// Internal to install.cc, inside namespace ibex2::jsi_adapter.
#pragma once

namespace {
void freeze(jsi::Runtime&, const jsi::Object&);

constexpr Groups kKnownGroups = GROUP_PURE | GROUP_CONSOLE | GROUP_TIMERS |
    GROUP_ABORT | GROUP_CRYPTO | GROUP_FETCH | GROUP_STORAGE | GROUP_ENV |
    GROUP_SECRETS | GROUP_KV | GROUP_INTL | GROUP_EVENTS | GROUP_BLOB |
    GROUP_WEBSOCKET;

bool has(Groups groups, Groups group) { return (groups & group) == group; }

void validate_groups_impl(Groups groups) {
  if ((groups & ~kKnownGroups) != 0)
    throw std::invalid_argument("unknown Ibex2 binding group bit");
  struct Requirement { Groups group; Groups required; };
  constexpr Requirement requirements[] = {
      {GROUP_TIMERS, GROUP_CONSOLE},
      {GROUP_ABORT, GROUP_PURE},
      {GROUP_CRYPTO, GROUP_PURE},
      {GROUP_FETCH, GROUP_PURE | GROUP_ABORT},
      {GROUP_EVENTS, GROUP_PURE},
      {GROUP_WEBSOCKET, GROUP_PURE | GROUP_EVENTS},
      {GROUP_BLOB, GROUP_PURE},
  };
  for (const auto& requirement : requirements) {
    if (has(groups, requirement.group) && !has(groups, requirement.required))
      throw std::invalid_argument("Ibex2 binding group is missing a dependency");
  }
#if !defined(IBEX2_JSI_HAS_INTL)
  if (has(groups, GROUP_INTL))
    throw std::invalid_argument("Ibex2 INTL bindings are unavailable in this build");
#elif defined(_WIN32)
  // @ref LLP 0057.000#511-windows-intl-uses-the-os-icu — the same probe as
  // Groups::validate: the OS must export the ICU the shims call.
  if (has(groups, GROUP_INTL) && ibex2_intl_os_icu_available() == 0)
    throw std::invalid_argument(
        "Ibex2 INTL bindings are unavailable: this Windows lacks the OS ICU "
        "they use (icu.dll from Windows 10 version 2004 or later)");
#endif
}

std::vector<const char*> expected_scripts_impl(Groups groups) {
  std::vector<const char*> result;
  if (has(groups, GROUP_PURE)) result.push_back("headers");
  if (has(groups, GROUP_TIMERS)) result.push_back("timers");
  if (has(groups, GROUP_PURE)) {
    result.push_back("url");
    result.push_back("domexception");
  }
  if (has(groups, GROUP_CRYPTO)) result.push_back("crypto");
  if (has(groups, GROUP_EVENTS)) result.push_back("events");
  if (has(groups, GROUP_ABORT)) result.push_back("abort");
  if (has(groups, GROUP_BLOB)) result.push_back("blob");
  if (has(groups, GROUP_WEBSOCKET)) result.push_back("websocket");
#if defined(IBEX2_JSI_HAS_INTL)
  if (has(groups, GROUP_INTL)) {
    result.push_back("intl_number_format");
    result.push_back("intl_case");
    result.push_back("intl_datetime");
  }
#endif
  if (has(groups, GROUP_FETCH)) result.push_back("fetch");
  if (has(groups, GROUP_STORAGE)) result.push_back("sqlite");
  if (has(groups, GROUP_PURE)) result.push_back("structured_clone");
  return result;
}

constexpr uint8_t kHermesBytecodeMagic[] = {
    0xc6, 0x1f, 0xbc, 0x03, 0xc1, 0x03, 0x19, 0x1f};

// The installed Hermes bundle exposes only public headers, not the internal
// file-format header. Full source builds use sizeof directly; the fallback is
// the selected pin's packed, cache-aligned BytecodeFileHeader size. Keep the
// assertion so a source-header build makes a pin change fail loudly.
#if defined(IBEX2_HAS_HERMES_BYTECODE_FILE_FORMAT)
constexpr size_t kHermesBytecodeHeaderSize =
    sizeof(::hermes::hbc::BytecodeFileHeader);
static_assert(kHermesBytecodeHeaderSize == 128,
              "update the installed-header BytecodeFileHeader size");
#else
constexpr size_t kHermesBytecodeHeaderSize = 128;
#endif

uint32_t bytecode_version(const CompiledScript& script) {
  return static_cast<uint32_t>(script.bytes[8]) |
      (static_cast<uint32_t>(script.bytes[9]) << 8) |
      (static_cast<uint32_t>(script.bytes[10]) << 16) |
      (static_cast<uint32_t>(script.bytes[11]) << 24);
}

uint32_t bytecode_declared_length(const CompiledScript& script) {
  return static_cast<uint32_t>(script.bytes[32]) |
      (static_cast<uint32_t>(script.bytes[33]) << 8) |
      (static_cast<uint32_t>(script.bytes[34]) << 16) |
      (static_cast<uint32_t>(script.bytes[35]) << 24);
}

// Header checks shared by install() and harden(): only precompiled Hermes
// bytecode of this runtime's version is ever evaluated by the adapter.
void validate_bytecode(const CompiledScript& script, uint32_t expected_version) {
  if (script.len < kHermesBytecodeHeaderSize)
    throw std::invalid_argument(
        "Ibex2 binding payload has a truncated Hermes bytecode header");
  if (std::memcmp(script.bytes, kHermesBytecodeMagic,
                  sizeof(kHermesBytecodeMagic)) != 0)
    throw std::invalid_argument("Ibex2 binding payload is not Hermes bytecode");
  if (bytecode_declared_length(script) != script.len)
    throw std::invalid_argument(
        "Ibex2 binding bytecode declared length does not match its buffer");
  if (expected_version != 0 && bytecode_version(script) != expected_version)
    throw std::invalid_argument(
        "Ibex2 binding bytecode version does not match the runtime");
}

struct ResponseOwner final : jsi::NativeState {
  void* owner;
  explicit ResponseOwner(void* value) : owner(value) {}
  ~ResponseOwner() override { ibex2_response_owner_destroy(owner); }
};

struct HeadersOwner final : jsi::NativeState {
  void* owner;
  explicit HeadersOwner(void* value) : owner(value) {}
  ~HeadersOwner() override { ibex2_headers_owner_destroy(owner); }
};

struct CryptoKeyOwner final : jsi::NativeState {
  void* owner;
  explicit CryptoKeyOwner(void* value) : owner(value) {}
  ~CryptoKeyOwner() override { ibex2_crypto_key_owner_destroy(owner); }
};

// Each opt-in fetch-primitives object is its own handle domain. The Rust
// registries remain runtime-wide, so these sets are the native, unforgeable
// proof that a handle crossed this exact bootstrap object. Handles are never
// recycled by RuntimeState, and terminal/released entries are removed here.
struct FetchPrimitiveHandles {
  std::unordered_set<uint64_t> responses;
  std::unordered_set<uint64_t> controls;
  std::unordered_set<uint64_t> headers;
};

[[noreturn]] void throw_primitive_error(jsi::Runtime& rt, const char* constructor,
                                        const std::string& message) {
  auto error = rt.global().getPropertyAsFunction(rt, constructor)
      .callAsConstructor(rt, message);
  throw jsi::JSError(rt, std::move(error));
}

[[noreturn]] void throw_primitive_type_error(jsi::Runtime& rt,
                                             const std::string& message) {
  throw_primitive_error(rt, "TypeError", message);
}

[[noreturn]] void throw_primitive_range_error(jsi::Runtime& rt,
                                              const std::string& message) {
  throw_primitive_error(rt, "RangeError", message);
}

// Every numeric argument crosses through here before any conversion: a
// non-number is a TypeError; NaN, an infinity, a fraction, or a value outside
// [min, max] is a RangeError. Only then is the double converted, so no
// out-of-range floating-to-integer conversion (undefined behavior) can happen.
uint64_t primitive_integer(jsi::Runtime& rt, const jsi::Value& value,
                           double min, double max, const char* what) {
  if (!value.isNumber())
    throw_primitive_type_error(rt, std::string(what) + " must be a number");
  const double number = value.getNumber();
  if (!std::isfinite(number) || std::floor(number) != number ||
      number < min || number > max)
    throw_primitive_range_error(
        rt, std::string(what) + " must be an integer from " +
                std::to_string(static_cast<uint64_t>(min)) + " to " +
                std::to_string(static_cast<uint64_t>(max)));
  return static_cast<uint64_t>(number);
}

constexpr double kMaxSafeInteger = 9007199254740991.0;

// The only accepted spelling of a trusted-bootstrap global. Restricting it to
// an ASCII identifier makes the Rust string, this C++ string, and the
// JavaScript property key the same bytes, so publication, collision checks,
// and the harden guard can never disagree about which property they mean.
bool is_ascii_identifier(const std::string& name) {
  auto start = [](char c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == '_' ||
        c == '$';
  };
  if (name.empty() || !start(name[0])) return false;
  for (char c : name)
    if (!start(c) && !(c >= '0' && c <= '9')) return false;
  return true;
}

// Every lookup of a chosen name goes through this one key construction.
jsi::PropNameID bootstrap_output_key(jsi::Runtime& rt, const std::string& name) {
  return jsi::PropNameID::forAscii(rt, name);
}

// A handle is an integer from 1 to 2^53 - 1 (Number.MAX_SAFE_INTEGER).
uint64_t primitive_handle(jsi::Runtime& rt, const jsi::Value& value,
                          const char* what) {
  return primitive_integer(rt, value, 1.0, kMaxSafeInteger, what);
}

// A well-formed handle that this primitives object did not create, or has
// already seen released, is a TypeError.
void require_owned(jsi::Runtime& rt, uint64_t handle,
                   const std::unordered_set<uint64_t>& owned, const char* what) {
  if (owned.find(handle) == owned.end())
    throw_primitive_type_error(
        rt, std::string(what) +
                " was not created by these fetch primitives or was already released");
}

uint64_t require_primitive_handle(
    jsi::Runtime& rt, const jsi::Value& value,
    const std::unordered_set<uint64_t>& owned, const char* what) {
  const uint64_t handle = primitive_handle(rt, value, what);
  require_owned(rt, handle, owned, what);
  return handle;
}

// The response fields the ABI defines (ibex2_response_field); 8 cancels.
bool known_response_field(uint64_t field) {
  switch (field) {
    case 0: case 1: case 2: case 3: case 5: case 7: case 8: return true;
    default: return false;
  }
}

jsi::Function make_group_binding(jsi::Runtime& rt, const char* name,
                                 uint32_t op,
                                 std::shared_ptr<Lifetime> lifetime) {
  auto prop = jsi::PropNameID::forUtf8(rt, std::string(name));
  return jsi::Function::createFromHostFunction(
      rt, prop, 1,
      [op, lifetime = std::move(lifetime)](
          jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
          size_t count) -> jsi::Value {
        const void* state = lifetime->require(r);
        return call_host(r, state, op, args, count);
      });
}

void set_group_binding(jsi::Runtime& rt, jsi::Object& target,
                       const char* name, uint32_t op,
                       const std::shared_ptr<Lifetime>& lifetime) {
  target.setProperty(rt, jsi::PropNameID::forUtf8(rt, std::string(name)),
                     make_group_binding(rt, name, op, lifetime));
}

void install_console(jsi::Runtime& rt,
                     const std::shared_ptr<Lifetime>& lifetime) {
  jsi::Object console(rt);
  set_group_binding(rt, console, "log", 1, lifetime);
  set_group_binding(rt, console, "info", 2, lifetime);
  set_group_binding(rt, console, "debug", 3, lifetime);
  set_group_binding(rt, console, "warn", 4, lifetime);
  set_group_binding(rt, console, "error", 5, lifetime);
  rt.global().setProperty(rt, "console", std::move(console));
}

void install_events(jsi::Runtime& rt,
                    const std::shared_ptr<Lifetime>& lifetime) {
  auto report = jsi::Function::createFromHostFunction(
      rt, jsi::PropNameID::forAscii(rt, "__ibex2_report_error"), 1,
      [lifetime](jsi::Runtime& r, const jsi::Value&,
                 const jsi::Value* args, size_t count) -> jsi::Value {
        lifetime->require(r);
        std::string message = count == 0 ? "uncaught error"
            : args[0].toString(r).utf8(r);
        ibex2_report_uncaught(message.c_str());
        return jsi::Value::undefined();
      });
  rt.global().setProperty(rt, "__ibex2_report_error", std::move(report));
}

void install_pure(jsi::Runtime& rt,
                  const std::shared_ptr<Lifetime>& lifetime) {
  auto global = rt.global();
  set_group_binding(rt, global, "__ibex2_text_encode", 20, lifetime);
  set_group_binding(rt, global, "__ibex2_text_decode", 21, lifetime);
  set_group_binding(rt, global, "__ibex2_text_encode_into", 22, lifetime);
  set_group_binding(rt, global, "__ibex2_url_parse", 30, lifetime);
  set_group_binding(rt, global, "__ibex2_url_set", 32, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_normalize", 29, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_get", 31, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_get_all", 33, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_has", 34, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_set", 35, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_append", 36, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_delete", 37, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_sort", 38, lifetime);
  set_group_binding(rt, global, "__ibex2_search_params_entries", 39, lifetime);

  jsi::Object headers(rt);
  set_group_binding(rt, headers, "create", 40, lifetime);
  set_group_binding(rt, headers, "append", 41, lifetime);
  set_group_binding(rt, headers, "set", 42, lifetime);
  set_group_binding(rt, headers, "get", 43, lifetime);
  set_group_binding(rt, headers, "has", 44, lifetime);
  set_group_binding(rt, headers, "remove", 45, lifetime);
  set_group_binding(rt, headers, "count", 46, lifetime);
  set_group_binding(rt, headers, "nameAt", 47, lifetime);
  set_group_binding(rt, headers, "valueAt", 48, lifetime);
  set_group_binding(rt, headers, "validName", 49, lifetime);
  set_group_binding(rt, headers, "validValue", 50, lifetime);
  set_group_binding(rt, headers, "free", 51, lifetime);
  headers.setProperty(
      rt, "own",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "ownHeaders"), 2,
          [lifetime](jsi::Runtime& r, const jsi::Value&,
                     const jsi::Value* args, size_t count) -> jsi::Value {
            const void* state = lifetime->require(r);
            if (count != 2 || !args[0].isNumber() || !args[1].isObject())
              throw jsi::JSError(r, "Headers owner needs a handle and object");
            void* owner = ibex2_headers_owner_create(state, args[0].asNumber());
            if (owner == nullptr)
              throw jsi::JSError(r, "Headers handle is released or unknown");
            set_native_state_once(r, args[1].getObject(r),
                                  std::make_shared<HeadersOwner>(owner),
                                  "Headers owner");
            return jsi::Value::undefined();
          }));
  global.setProperty(rt, "__ibex2_headers", std::move(headers));
}

void install_timers(jsi::Runtime& rt,
                    const std::shared_ptr<Lifetime>& lifetime) {
  auto global = rt.global();
  set_group_binding(rt, global, "__ibex2_timer_set", 60, lifetime);
  set_group_binding(rt, global, "__ibex2_timer_set_repeating", 61, lifetime);
  set_group_binding(rt, global, "__ibex2_timer_clear", 62, lifetime);
  set_group_binding(rt, global, "__ibex2_performance_now", 63, lifetime);
}

void install_crypto(jsi::Runtime& rt,
                    const std::shared_ptr<Lifetime>& lifetime) {
  auto global = rt.global();
  set_group_binding(rt, global, "__ibex2_random_uuid", 70, lifetime);
  set_group_binding(rt, global, "__ibex2_get_random_values", 71, lifetime);
  jsi::Object subtle(rt);
  set_group_binding(rt, subtle, "digest", 160, lifetime);
  set_group_binding(rt, subtle, "importKey", 161, lifetime);
  set_group_binding(rt, subtle, "exportKey", 162, lifetime);
  set_group_binding(rt, subtle, "generateKey", 163, lifetime);
  set_group_binding(rt, subtle, "sign", 164, lifetime);
  set_group_binding(rt, subtle, "verify", 165, lifetime);
  set_group_binding(rt, subtle, "encrypt", 166, lifetime);
  set_group_binding(rt, subtle, "decrypt", 167, lifetime);
  set_group_binding(rt, subtle, "deriveBits", 168, lifetime);
  set_group_binding(rt, subtle, "deriveKey", 169, lifetime);
  subtle.setProperty(
      rt, "own",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "ownCryptoKey"), 2,
          [lifetime](jsi::Runtime& r, const jsi::Value&,
                     const jsi::Value* args, size_t count) -> jsi::Value {
            const void* state = lifetime->require(r);
            if (count != 2 || !args[0].isNumber() || !args[1].isObject())
              throw jsi::JSError(r, "CryptoKey owner needs a handle and object");
            void* owner = ibex2_crypto_key_owner_create(state, args[0].asNumber());
            if (owner == nullptr)
              throw jsi::JSError(r, "CryptoKey handle is released or unknown");
            set_native_state_once(r, args[1].getObject(r),
                                  std::make_shared<CryptoKeyOwner>(owner),
                                  "CryptoKey owner");
            return jsi::Value::undefined();
          }));
  global.setProperty(rt, "__ibex2_subtle", std::move(subtle));
}

void install_blob(jsi::Runtime& rt,
                  const std::shared_ptr<Lifetime>& lifetime) {
  auto global = rt.global();
  set_group_binding(rt, global, "__ibex2_multipart_boundary", 73, lifetime);
  set_group_binding(rt, global, "__ibex2_multipart_encode", 74, lifetime);
}

jsi::Function make_response_field(
    jsi::Runtime& rt, const std::shared_ptr<Lifetime>& lifetime,
    std::shared_ptr<FetchPrimitiveHandles> primitive_handles = nullptr) {
  return jsi::Function::createFromHostFunction(
      rt, jsi::PropNameID::forAscii(rt, "__ibex2_response_field"), 3,
      [lifetime, primitive_handles = std::move(primitive_handles)](
          jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
          size_t count) -> jsi::Value {
        const void* queue = lifetime->require(r);
        if (count < 2)
          throw_primitive_type_error(r, "response field needs a handle and a field id");
        // Shapes first, ownership second: nothing below converts an
        // unvalidated double.
        if (!args[0].isNumber())
          throw_primitive_type_error(r, "response handle must be a number");
        const uint64_t field =
            primitive_integer(r, args[1], 0.0, 4294967295.0, "response field id");
        uint64_t primitive_response = 0;
        if (primitive_handles != nullptr) {
          primitive_response = primitive_handle(r, args[0], "response handle");
          if (!known_response_field(field))
            throw_primitive_range_error(
                r, "unknown response field " + std::to_string(field));
          if (field == 3 && (count < 3 || !args[2].isString()))
            throw_primitive_type_error(r, "response header name must be a string");
          require_owned(r, primitive_response, primitive_handles->responses,
                        "response handle");
        }
        std::vector<std::string> owned;
        Ibex2AbiValue name{IBEX2_TAG_UNDEFINED, 0.0, nullptr, 0};
        if (count >= 3) name = to_abi(r, args[2], owned);
        Ibex2AbiValue out{IBEX2_TAG_UNDEFINED, 0.0, nullptr, 0};
        int status = ibex2_response_field(
            queue, args[0].getNumber(), static_cast<uint32_t>(field),
            count >= 3 ? &name : nullptr, &out);
        struct Release {
          Ibex2AbiValue& value;
          ~Release() { ibex2_host_release(&value); }
        } release{out};
        auto result = from_abi(r, out);
        if (status != 0) {
          // Arguments were validated above, so for the primitives the one
          // remaining failure is a row the ABI no longer has.
          if (primitive_handles != nullptr)
            primitive_handles->responses.erase(primitive_response);
          throw jsi::JSError(r, result.isString()
              ? result.getString(r).utf8(r) : std::string("response read failed"));
        }
        if (primitive_handles != nullptr && field == 8)
          primitive_handles->responses.erase(primitive_response);
        return result;
      });
}

void install_fetch(jsi::Runtime& rt, Adapter& adapter,
                   const std::shared_ptr<Lifetime>& lifetime) {
  auto global = rt.global();
  set_group_binding(rt, global, "__ibex2_fetch_control", 72, lifetime);
  global.setProperty(rt, "__ibex2_response_own",
      jsi::Function::createFromHostFunction(rt,
          jsi::PropNameID::forAscii(rt, "__ibex2_response_own"), 2,
          [lifetime](jsi::Runtime& r, const jsi::Value&,
                     const jsi::Value* args, size_t count) -> jsi::Value {
            const void* queue = lifetime->require(r);
            if (count != 2 || !args[0].isNumber() || !args[1].isObject())
              throw jsi::JSError(r, "response owner needs a handle and a body");
            auto body = args[1].getObject(r);
            set_native_state_once(r, body, std::make_shared<ResponseOwner>(
                ibex2_response_owner_create(queue, args[0].asNumber())),
                "response body owner");
            auto weak = std::make_shared<jsi::WeakObject>(r, body);
            return jsi::Function::createFromHostFunction(r,
                jsi::PropNameID::forAscii(r, "responseBody"), 0,
                [weak](jsi::Runtime& r, const jsi::Value&, const jsi::Value*,
                       size_t) -> jsi::Value { return weak->lock(r); });
          }));
  global.setProperty(rt, "__ibex2_response_read",
                     adapter.async_binding("__ibex2_response_read", 102, nullptr));
  global.setProperty(rt, "__ibex2_response_field",
                     make_response_field(rt, lifetime));
}

jsi::Object make_fetch_primitives(
    jsi::Runtime& rt, Adapter& adapter,
    const std::shared_ptr<Lifetime>& lifetime, const void* grants) {
  auto handles = std::make_shared<FetchPrimitiveHandles>();
  jsi::Object primitives(rt);
  auto raw_fetch = std::make_shared<jsi::Function>(
      adapter.async_binding("fetch", 101, grants));
  primitives.setProperty(
      rt, "fetch",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "fetch"), 6,
          [lifetime, handles, raw_fetch](
              jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
              size_t count) -> jsi::Value {
            const void* state = lifetime->require(r);
            // Exactly six arguments cross; a missing one is undefined.
            std::vector<jsi::Value> call;
            call.reserve(6);
            for (size_t i = 0; i < 6; ++i)
              call.push_back(i < count ? jsi::Value(r, args[i])
                                       : jsi::Value::undefined());
            // The headers handle is validated and adopted first, so once it
            // is accepted it is releasable through headersFree whatever this
            // call does next -- including throwing below.
            if (!call[4].isUndefined()) {
              const uint64_t header =
                  primitive_handle(r, call[4], "fetch headers handle");
              try {
                call_host(r, state, 46, &call[4], 1);
              } catch (const jsi::JSError&) {
                throw_primitive_type_error(r, "unknown fetch headers handle");
              }
              handles->headers.insert(header);
            }
            if (!call[0].isString())
              throw_primitive_type_error(r, "fetch url must be a string");
            if (!call[1].isUndefined() && !call[1].isString())
              throw_primitive_type_error(r, "fetch method must be a string or undefined");
            if (!call[2].isUndefined() && !call[2].isNull()) {
              const bool bytes = call[2].isObject() &&
                  (call[2].getObject(r).isArrayBuffer(r) ||
                   call[2].getObject(r).isTypedArray(r));
              if (!bytes)
                throw_primitive_type_error(
                    r, "fetch body must be undefined, null, an ArrayBuffer, or a typed array");
            }
            if (!call[3].isUndefined() && !call[3].isString())
              throw_primitive_type_error(r, "fetch redirect must be a string or undefined");
            if (!call[5].isUndefined())
              require_primitive_handle(r, call[5], handles->controls,
                                       "fetch control token");
            auto promise =
                raw_fetch->call(r, static_cast<const jsi::Value*>(call.data()),
                                call.size())
                    .getObject(r);
            auto own_response = jsi::Function::createFromHostFunction(
                r, jsi::PropNameID::forAscii(r, "ownFetchResponse"), 1,
                [lifetime, handles](
                    jsi::Runtime& r, const jsi::Value&,
                    const jsi::Value* args, size_t count) -> jsi::Value {
                  lifetime->require(r);
                  if (count != 1)
                    throw_primitive_type_error(r, "fetch returned no response handle");
                  const uint64_t response =
                      primitive_handle(r, args[0], "fetch response handle");
                  handles->responses.insert(response);
                  return jsi::Value(r, args[0]);
                });
            return promise.getPropertyAsFunction(r, "then").callWithThis(
                r, promise, std::move(own_response));
          }));
  primitives.setProperty(rt, "responseField",
      make_response_field(rt, lifetime, handles));
  auto raw_read = std::make_shared<jsi::Function>(
      adapter.async_binding("responseRead", 102, nullptr));
  primitives.setProperty(
      rt, "responseRead",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "responseRead"), 1,
          [lifetime, handles, raw_read](
              jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
              size_t count) -> jsi::Value {
            lifetime->require(r);
            if (count < 1)
              throw_primitive_type_error(r, "response handle is required");
            const uint64_t response = require_primitive_handle(
                r, args[0], handles->responses, "response handle");
            auto promise = raw_read->call(r, args, size_t{1}).getObject(r);
            auto complete = jsi::Function::createFromHostFunction(
                r, jsi::PropNameID::forAscii(r, "completeResponseRead"), 1,
                [lifetime, handles, response](
                    jsi::Runtime& r, const jsi::Value&,
                    const jsi::Value* values, size_t value_count) -> jsi::Value {
                  lifetime->require(r);
                  if (value_count != 1)
                    throw_primitive_type_error(r, "response read returned no value");
                  if (values[0].isNull()) handles->responses.erase(response);
                  return jsi::Value(r, values[0]);
                });
            auto fail = jsi::Function::createFromHostFunction(
                r, jsi::PropNameID::forAscii(r, "failResponseRead"), 1,
                [lifetime, handles, response](
                    jsi::Runtime& r, const jsi::Value&,
                    const jsi::Value* values, size_t value_count) -> jsi::Value {
                  lifetime->require(r);
                  handles->responses.erase(response);
                  if (value_count == 1)
                    throw jsi::JSError(r, jsi::Value(r, values[0]));
                  throw jsi::JSError(r, "response read failed");
                });
            return promise.getPropertyAsFunction(r, "then").callWithThis(
                r, promise, std::move(complete), std::move(fail));
          }));
  primitives.setProperty(
      rt, "fetchControl",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "fetchControl"), 2,
          [lifetime, handles](
              jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
              size_t count) -> jsi::Value {
            const void* state = lifetime->require(r);
            if (count < 1)
              throw_primitive_type_error(r, "fetch control action is required");
            // 0 allocates, 1 aborts, 2 releases; nothing else crosses.
            const uint64_t action =
                primitive_integer(r, args[0], 0.0, 2.0, "fetch control action");
            if (action == 0) {
              auto result = call_host(r, state, 72, args, 1);
              handles->controls.insert(
                  primitive_handle(r, result, "fetch control token"));
              return result;
            }
            if (count < 2)
              throw_primitive_type_error(r, "fetch control token is required");
            const uint64_t token = require_primitive_handle(
                r, args[1], handles->controls, "fetch control token");
            auto result = call_host(r, state, 72, args, 2);
            if (action == 2) handles->controls.erase(token);
            return result;
          }));
  primitives.setProperty(rt, "textEncode",
      make_group_binding(rt, "textEncode", 20, lifetime));
  primitives.setProperty(rt, "textDecode",
      make_group_binding(rt, "textDecode", 21, lifetime));
  primitives.setProperty(rt, "textEncodeInto",
      make_group_binding(rt, "textEncodeInto", 22, lifetime));
  primitives.setProperty(
      rt, "headersFree",
      jsi::Function::createFromHostFunction(
          rt, jsi::PropNameID::forAscii(rt, "headersFree"), 1,
          [lifetime, handles](
              jsi::Runtime& r, const jsi::Value&, const jsi::Value* args,
              size_t count) -> jsi::Value {
            const void* state = lifetime->require(r);
            if (count < 1)
              throw_primitive_type_error(r, "headers handle is required");
            const uint64_t header = require_primitive_handle(
                r, args[0], handles->headers, "headers handle");
            auto result = call_host(r, state, 51, args, 1);
            handles->headers.erase(header);
            return result;
          }));
  freeze(rt, primitives);
  return primitives;
}

// The members of the published object, in the order their identities are
// recorded after the object's own identity.
constexpr const char* kFetchPrimitiveMembers[] = {
    "fetch", "responseField", "responseRead", "fetchControl",
    "textEncode", "textDecode", "textEncodeInto", "headersFree"};

// Bounds the production guard on a pathological graph; reaching it refuses
// (fails closed). Tests use a much smaller object limit and an explicit edge
// limit below so the privacy witness remains a fast regression.
constexpr size_t kMaxReachabilityObjects = size_t{1} << 20;

// Walk what harden.js freezes and refuse if any recorded identity is in it.
//
// Coverage mirrors harden.js exactly: the global object's own properties
// (string and symbol keys), its prototype chain, and transitively every object
// or function reached through an own property -- a data property's value, or
// an accessor's getter and setter functions themselves -- plus each reached
// object's prototype. Descriptors are read with the captured
// Object.getOwnPropertyDescriptor, so no getter is ever invoked. A Set of
// visited objects makes the walk cycle-safe; the explicit stack keeps it off
// the native stack; kMaxReachabilityObjects bounds it.
//
// Not covered, because harden.js cannot see it either: values held only in
// closure scopes (including a getter's closure), in native state, in
// WeakMap/Map/Set entries, behind a Proxy whose traps conceal them, or in an
// object reachable only from somewhere other than the global object. Those
// stay the trusted bootstrap's obligation.
// @ref LLP 0068#opt-in-fetch-primitives-protocol — L1e: the harden guard proves the bootstrap handoff did not leave a path to the primitives
// @ref LLP 0068#opt-in-abort-hooks-protocol — I4 applies the same reachability proof to the abort hooks
void require_unreachable(
    jsi::Runtime& rt, const Reachability& walk,
    const std::vector<std::pair<std::string, jsi::Value>>& identities,
    size_t max_objects = kMaxReachabilityObjects,
    size_t max_edges = std::numeric_limits<size_t>::max()) {
  try {
    auto global = rt.global();
    auto seen = walk.set.callAsConstructor(rt).getObject(rt);
    std::vector<jsi::Value> pending;
    size_t edges = 0;
    // Made once: descriptor records have ordinary own data fields, while
    // creating field strings or IDs for every edge allocates in Hermes.
    const auto value_key = jsi::String::createFromAscii(rt, "value");
    const auto value_name = jsi::PropNameID::forAscii(rt, "value");
    const auto get_name = jsi::PropNameID::forAscii(rt, "get");
    const auto set_name = jsi::PropNameID::forAscii(rt, "set");
    auto expand = [&](const jsi::Object& object) {
      for (const auto* list : {&walk.names, &walk.symbols}) {
        auto keys = list->call(rt, object).getObject(rt).getArray(rt);
        const size_t count = keys.size(rt);
        for (size_t i = 0; i < count; ++i) {
          if (edges >= max_edges)
            throw std::runtime_error(
                "refusing to harden: the global object graph has too many "
                "edges to prove trusted-bootstrap outputs unreachable");
          ++edges;
          auto raw = walk.descriptor.call(rt, object, keys.getValueAtIndex(rt, i));
          if (!raw.isObject()) continue;
          auto descriptor = raw.getObject(rt);
          // Classify by the descriptor record's OWN fields with the captured
          // hasOwnProperty. The record is the ordinary FromPropertyDescriptor
          // result, so reading a field after that test invokes no user code.
          if (walk.has_own.callWithThis(rt, descriptor, value_key).getBool()) {
            pending.push_back(descriptor.getProperty(rt, value_name));
          } else {
            pending.push_back(descriptor.getProperty(rt, get_name));
            pending.push_back(descriptor.getProperty(rt, set_name));
          }
        }
      }
      if (edges >= max_edges)
        throw std::runtime_error(
            "refusing to harden: the global object graph has too many edges "
            "to prove trusted-bootstrap outputs unreachable");
      ++edges;
      pending.push_back(walk.prototype.call(rt, object));
    };
    walk.add.callWithThis(rt, seen, global);
    expand(global);
    size_t visited = 0;
    while (!pending.empty()) {
      jsi::Value value = std::move(pending.back());
      pending.pop_back();
      if (!value.isObject()) continue;
      if (walk.has.callWithThis(rt, seen, value).getBool()) continue;
      walk.add.callWithThis(rt, seen, value);
      if (++visited > max_objects)
        throw std::runtime_error(
            "refusing to harden: the global object graph is too large to "
            "prove trusted-bootstrap outputs unreachable");
      auto object = value.getObject(rt);
      for (const auto& [label, identity] : identities) {
        if (jsi::Object::strictEquals(rt, object, identity.getObject(rt)))
          throw std::runtime_error(
              "refusing to harden: " + label +
              " is still reachable from the global object");
      }
      expand(object);
    }
  } catch (const jsi::JSIException& error) {
    throw std::runtime_error(
        std::string("refusing to harden: could not prove trusted-bootstrap outputs "
                    "unreachable: ") + error.what());
  }
}

jsi::Object make_process(jsi::Runtime& rt, const void* grants) {
  jsi::Object env(rt);
  const size_t count = ibex2_grants_env_count(grants);
  for (size_t i = 0; i < count; ++i) {
    char* name = nullptr;
    char* value = nullptr;
    if (ibex2_grants_env_at(grants, i, &name, &value) == 0) continue;
    env.setProperty(rt, jsi::PropNameID::forUtf8(rt, std::string(name)),
                    jsi::String::createFromUtf8(rt, std::string(value)));
    ibex2_string_free(name);
    ibex2_string_free(value);
  }
  freeze(rt, env);
  jsi::Object process(rt);
  process.setProperty(rt, "env", std::move(env));
  freeze(rt, process);
  return process;
}

void remove_global(jsi::Runtime& rt, const jsi::Object& global,
                   const char* name) {
  rt.global().getPropertyAsObject(rt, "Reflect")
      .getPropertyAsFunction(rt, "deleteProperty")
      .call(rt, global, jsi::String::createFromUtf8(rt, name));
}

uint64_t websocket_handle(jsi::Runtime& rt, const jsi::Value& value) {
  return primitive_integer(
      rt, value, 1.0, kMaxSafeInteger, "WebSocket handle");
}

void websocket_result(jsi::Runtime& rt, int status, char* error,
                      const char* fallback) {
  std::string message = error == nullptr ? fallback : std::string(error);
  if (error != nullptr) ibex2_string_free(error);
  if (status != 0) throw jsi::JSError(rt, message);
}
} // namespace
