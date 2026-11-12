// Internal to install.cc, inside namespace ibex2::jsi_adapter.
#pragma once

// Convert a JS argument. Strings are decoded into `owned`, which the caller
// keeps alive for the duration of the host call so the span stays valid.
Ibex2AbiValue to_abi(jsi::Runtime &rt, const jsi::Value &value,
                     std::vector<std::string> &owned) {
  Ibex2AbiValue out{IBEX2_TAG_UNDEFINED, 0.0, nullptr, 0};
  if (value.isUndefined()) {
    return out;
  }
  if (value.isNull()) {
    out.tag = IBEX2_TAG_NULL;
    return out;
  }
  if (value.isBool()) {
    out.tag = IBEX2_TAG_BOOL;
    out.number = value.getBool() ? 1.0 : 0.0;
    return out;
  }
  if (value.isNumber()) {
    out.tag = IBEX2_TAG_NUMBER;
    out.number = value.getNumber();
    return out;
  }
  if (value.isObject() && value.getObject(rt).isArrayBuffer(rt)) {
    auto buffer = value.getObject(rt).getArrayBuffer(rt);
    out.tag = IBEX2_TAG_BYTES;
    out.data = buffer.data(rt);
    out.len = buffer.size(rt);
    return out;
  }
  // A TYPED ARRAY, which is what application code actually passes: a Uint8Array
  // is not an ArrayBuffer, and handling only the latter made
  // `fs.writeFile(path, new TextEncoder().encode(text))` stringify its payload
  // and write an empty file. The view's offset matters — a subarray shares its
  // buffer with the whole, so reading from the buffer's start would send the
  // wrong bytes.
  if (value.isObject() && value.getObject(rt).isTypedArray(rt)) {
    auto view = value.getObject(rt).getTypedArray(rt);
    auto buffer = view.buffer(rt);
    out.tag = IBEX2_TAG_BYTES;
    out.data = buffer.data(rt) + view.byteOffset(rt);
    out.len = view.byteLength(rt);
    return out;
  }
  // Everything else stringifies, which is what console does with its arguments.
  owned.push_back(value.toString(rt).utf8(rt));
  const std::string &text = owned.back();
  out.tag = IBEX2_TAG_STRING;
  out.data = reinterpret_cast<const unsigned char *>(text.data());
  out.len = text.size();
  return out;
}

// A byte result IS the JavaScript ArrayBuffer's storage — no copy, no second
// allocation. Rust allocated it, ownership transfers here, and the engine frees
// it back through the boundary when the ArrayBuffer is collected. That is the
// outbound half of LLP 0059.000 §1.2.
//
// The destructor is the whole mechanism: Hermes holds this shared_ptr for as
// long as the ArrayBuffer is reachable, so the Rust allocation outlives every
// JavaScript reference to it and is released exactly once.
class RustBytes : public jsi::MutableBuffer {
public:
  explicit RustBytes(Ibex2AbiValue value) : value_(value) {}
  ~RustBytes() override { ibex2_host_release(&value_); }

  RustBytes(const RustBytes &) = delete;
  RustBytes &operator=(const RustBytes &) = delete;

  size_t size() const override { return value_.len; }
  uint8_t *data() override {
    return const_cast<uint8_t *>(value_.data);
  }

private:
  Ibex2AbiValue value_;
};

// Engines may retain bytecode storage for as long as evaluated functions are
// live. A span aligned for Hermes's file header is read where the caller
// keeps it (`CompiledScript`: for the runtime's life); any other is copied,
// since Hermes refuses an unaligned buffer.
class CompiledBytes : public jsi::Buffer {
public:
  CompiledBytes(const uint8_t* data, size_t len) : data_(data), len_(len) {
    if (reinterpret_cast<uintptr_t>(data) % 16 != 0) {
      bytes_.assign(data, data + len);
      data_ = bytes_.data();
    }
  }
  size_t size() const override { return len_; }
  const uint8_t* data() const override { return data_; }
private:
  std::vector<uint8_t> bytes_;
  const uint8_t* data_;
  size_t len_;
};

// Convert a result. For bytes this TAKES OWNERSHIP and clears `value`, so the
// caller's release becomes a no-op — the RustBytes destructor releases instead,
// when the engine is done with the buffer. Strings still copy: Hermes owns its
// own string representation and there is no way to hand it one (§1.2).
jsi::Value from_abi(jsi::Runtime &rt, Ibex2AbiValue &value) {
  switch (value.tag) {
  case IBEX2_TAG_NULL:
    return jsi::Value::null();
  case IBEX2_TAG_BOOL:
    return jsi::Value(value.number != 0.0);
  case IBEX2_TAG_NUMBER:
    return jsi::Value(value.number);
  case IBEX2_TAG_STRING: {
    std::string text(reinterpret_cast<const char *>(value.data), value.len);
    return jsi::String::createFromUtf8(rt, text);
  }
  case IBEX2_TAG_BYTES: {
    Ibex2AbiValue owned = value;
    value = Ibex2AbiValue{IBEX2_TAG_UNDEFINED, 0.0, nullptr, 0};
    return jsi::Value(rt,
                      jsi::ArrayBuffer(rt, std::make_shared<RustBytes>(owned)));
  }
  default:
    return jsi::Value::undefined();
  }
}

// The result-bearing half is also used by bindings that need to translate a
// Rust-owned error kind into the matching JavaScript error constructor.
HostCallResult call_host_result(jsi::Runtime& rt, const void* state,
                                uint32_t op, const jsi::Value* args,
                                size_t count) {
  std::vector<std::string> owned;
  owned.reserve(count);
  std::vector<Ibex2AbiValue> abi;
  abi.reserve(count);
  for (size_t i = 0; i < count; ++i) {
    abi.push_back(to_abi(rt, args[i], owned));
  }
  Ibex2AbiValue out{IBEX2_TAG_UNDEFINED, 0.0, nullptr, 0};
  int status = ibex2_host_call(state, op,
                               abi.empty() ? nullptr : abi.data(),
                               abi.size(), &out);
  struct Release { Ibex2AbiValue& value; ~Release() { ibex2_host_release(&value); } } release{out};
  jsi::Value result = from_abi(rt, out);
  return HostCallResult{status, std::move(result)};
}

void set_native_state_once(jsi::Runtime& rt, const jsi::Object& object,
                           std::shared_ptr<jsi::NativeState> state,
                           const char* owner) {
  if (object.hasNativeState(rt))
    throw jsi::JSError(rt, std::string(owner) +
        " cannot replace an existing native owner");
  object.setNativeState(rt, std::move(state));
}

// All ordinary synchronous bindings share the same conversion and release
// path. Their public contracts report a generic host-call failure.
static jsi::Value call_host(jsi::Runtime& rt, const void* state, uint32_t op,
                           const jsi::Value* args, size_t count) {
  auto result = call_host_result(rt, state, op, args, count);
  if (result.status != 0) {
    // The Rust error taxonomy becomes a JS throw here, so failures are
    // identical on every platform (LLP 0057 §3).
    throw jsi::JSError(rt, result.value.isString()
                               ? result.value.getString(rt).utf8(rt)
                               : std::string("host call failed"));
  }
  return std::move(result.value);
}

// One host function per op, so JavaScript sees ordinary callables while every
// one of them funnels through the single ibex2_host_call surface.
jsi::Function make_host_binding(jsi::Runtime &runtime, const char *name,
                                uint32_t op, const void *state) {
  auto prop = jsi::PropNameID::forUtf8(runtime, std::string(name));
  return jsi::Function::createFromHostFunction(
      runtime, prop, 1,
      [op, state](jsi::Runtime &rt, const jsi::Value &, const jsi::Value *args,
                  size_t count) -> jsi::Value {
        return call_host(rt, state, op, args, count);
      });
}

void set_binding(jsi::Runtime &rt, jsi::Object &target, const char *name,
                 uint32_t op, const void *state) {
  target.setProperty(rt, jsi::PropNameID::forUtf8(rt, std::string(name)),
                     make_host_binding(rt, name, op, state));
}
