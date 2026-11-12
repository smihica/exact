// Internal to install.cc, inside namespace ibex2::jsi_adapter.
#pragma once

// Capture before application code. A later freeze alone is insufficient: an
// application could replace WeakMap.prototype.set first, then freeze it. Pin
// descriptors as well as identities so that cannot expose private SQL handles.
struct Integrity {
  struct Property {
    jsi::Value key, value, get, set;
  };
  struct Intrinsic {
    jsi::Object object;
    jsi::Value prototype;
    std::vector<Property> properties;
  };
  std::vector<std::pair<std::string, jsi::Value>> globals;
  std::vector<Intrinsic> intrinsics;
  jsi::Function descriptor, names, symbols;
  // Made once: a PropNameID made from text is a heap allocation, and every
  // descriptor read takes three.
  jsi::PropNameID value_name, get_name, set_name;
  jsi::Function prototype, frozen, same;
  bool validated = false;

  explicit Integrity(jsi::Runtime& rt)
      : descriptor(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "getOwnPropertyDescriptor")),
        names(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "getOwnPropertyNames")),
        symbols(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "getOwnPropertySymbols")),
        value_name(jsi::PropNameID::forAscii(rt, "value")),
        get_name(jsi::PropNameID::forAscii(rt, "get")),
        set_name(jsi::PropNameID::forAscii(rt, "set")),
        prototype(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "getPrototypeOf")),
        frozen(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "isFrozen")),
        same(rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, "is")) {
    for (const char* name : {"Object", "Function", "Array", "Promise", "WeakMap",
          "Reflect", "Number", "BigInt", "Uint8Array", "ArrayBuffer", "Error",
          "TypeError", "RangeError", "String", "JSON", "Symbol", "Map", "Set"}) {
      auto value = rt.global().getProperty(rt, name);
      if (!value.isObject()) throw jsi::JSError(rt, "Ibex2 SQLite requires standard intrinsics");
      globals.emplace_back(name, jsi::Value(rt, value));
      capture(rt, value.getObject(rt));
      auto d = descriptor.call(rt, value, "prototype");
      if (d.isObject()) {
        auto p = d.getObject(rt).getProperty(rt, "value");
        if (p.isObject()) capture(rt, p.getObject(rt));
      }
    }
  }

  std::vector<jsi::Value> keys(jsi::Runtime& rt, const jsi::Object& object) {
    std::vector<jsi::Value> result;
    for (const auto* function : {&names, &symbols}) {
      auto array = function->call(rt, object).getObject(rt).getArray(rt);
      for (size_t i = 0; i < array.size(rt); ++i) result.push_back(array.getValueAtIndex(rt, i));
    }
    return result;
  }

  void capture(jsi::Runtime& rt, jsi::Object object) {
    for (const auto& item : intrinsics)
      if (jsi::Object::strictEquals(rt, object, item.object)) return;
    auto parent = prototype.call(rt, object);
    Intrinsic item{std::move(object), jsi::Value(rt, parent), {}};
    for (auto& key : keys(rt, item.object)) {
      auto d = descriptor.call(rt, item.object, key).getObject(rt);
      item.properties.push_back(Property{std::move(key), d.getProperty(rt, value_name),
          d.getProperty(rt, get_name), d.getProperty(rt, set_name)});
    }
    intrinsics.push_back(std::move(item));
    if (parent.isObject()) capture(rt, parent.getObject(rt));
  }

  bool equal(jsi::Runtime& rt, const jsi::Value& a, const jsi::Value& b) {
    return same.call(rt, a, b).getBool();
  }

  void require(jsi::Runtime& rt, bool hardened = true) {
    if (validated && hardened) return;
    auto refuse = [&]() {
      throw jsi::JSError(rt, "Ibex2 SQLite requires unchanged, hardened intrinsics: create bindings before application code and harden through Adapter::harden() before using them");
    };
    for (const auto& [name, value] : globals) {
      auto d = descriptor.call(rt, rt.global(), jsi::String::createFromUtf8(rt, name));
      if (!d.isObject()) refuse();
      auto property = d.getObject(rt);
      if (!equal(rt, property.getProperty(rt, "value"), value)) refuse();
      if (hardened && (property.getProperty(rt, "writable").isUndefined()
          || property.getProperty(rt, "writable").getBool()
          || property.getProperty(rt, "configurable").getBool())) refuse();
    }
    for (const auto& item : intrinsics) {
      if ((hardened && !frozen.call(rt, item.object).getBool())
          || !equal(rt, prototype.call(rt, item.object), item.prototype)
          || keys(rt, item.object).size() != item.properties.size()) refuse();
      for (const auto& property : item.properties) {
        auto raw = descriptor.call(rt, item.object, property.key);
        if (!raw.isObject()) refuse();
        auto d = raw.getObject(rt);
        for (const auto& pair : {std::pair<const jsi::PropNameID*, const jsi::Value*>{&value_name, &property.value},
                                 {&get_name, &property.get}, {&set_name, &property.set}}) {
          if (!equal(rt, d.getProperty(rt, *pair.first), *pair.second)) refuse();
          if (hardened && pair.second->isObject()
              && !frozen.call(rt, *pair.second).getBool()) refuse();
        }
      }
    }
    validated = hardened;
  }

  void accept_property(jsi::Runtime& rt, const jsi::Object& object,
                       const char* name) {
    if (validated)
      throw jsi::JSError(rt, "cannot replace an intrinsic after validation");
    auto key = jsi::Value(rt, jsi::String::createFromUtf8(rt, name));
    for (auto& item : intrinsics) {
      if (!jsi::Object::strictEquals(rt, object, item.object)) continue;
      for (auto& property : item.properties) {
        if (!equal(rt, property.key, key)) continue;
        auto raw = descriptor.call(rt, item.object, key);
        if (!raw.isObject())
          throw jsi::JSError(rt, "trusted intrinsic replacement is absent");
        auto current = raw.getObject(rt);
        property.value = current.getProperty(rt, value_name);
        property.get = current.getProperty(rt, get_name);
        property.set = current.getProperty(rt, set_name);
        return;
      }
      throw jsi::JSError(rt, "trusted intrinsic property was not captured");
    }
    throw jsi::JSError(rt, "trusted intrinsic object was not captured");
  }
};

// The reflective intrinsics the trusted-bootstrap reachability walk uses,
// captured when install_with publishes an object -- before the embedder's
// bootstrap runs between install and harden -- so that bootstrap cannot change
// what the walk sees by replacing Object.getOwnPropertyNames or Set.prototype.
struct Reachability {
  jsi::Function names, symbols, descriptor, has_own, prototype, set, has, add;
  explicit Reachability(jsi::Runtime& rt)
      : names(object_function(rt, "getOwnPropertyNames")),
        symbols(object_function(rt, "getOwnPropertySymbols")),
        descriptor(object_function(rt, "getOwnPropertyDescriptor")),
        has_own(rt.global().getPropertyAsObject(rt, "Object")
                    .getPropertyAsObject(rt, "prototype")
                    .getPropertyAsFunction(rt, "hasOwnProperty")),
        prototype(object_function(rt, "getPrototypeOf")),
        set(rt.global().getPropertyAsFunction(rt, "Set")),
        has(set_method(rt, "has")),
        add(set_method(rt, "add")) {}
  static jsi::Function object_function(jsi::Runtime& rt, const char* name) {
    return rt.global().getPropertyAsObject(rt, "Object").getPropertyAsFunction(rt, name);
  }
  static jsi::Function set_method(jsi::Runtime& rt, const char* name) {
    return rt.global().getPropertyAsObject(rt, "Set")
        .getPropertyAsObject(rt, "prototype").getPropertyAsFunction(rt, name);
  }
};
