// Lean compiler output
// Module: Ferro.AuthTokenProperties
// Imports: Init
#include <lean/lean.h>
#if defined(__clang__)
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wunused-label"
#elif defined(__GNUC__) && !defined(__CLANG__)
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wunused-label"
#pragma GCC diagnostic ignored "-Wunused-but-set-variable"
#endif
#ifdef __cplusplus
extern "C" {
#endif
LEAN_EXPORT lean_object* l_AuthToken_refreshDuration;
LEAN_EXPORT lean_object* l_AuthToken_currentTime;
LEAN_EXPORT lean_object* l_AuthToken_refresh(lean_object*, lean_object*);
lean_object* lean_nat_add(lean_object*, lean_object*);
static lean_object* _init_l_AuthToken_currentTime() {
_start:
{
lean_object* x_1; 
x_1 = lean_unsigned_to_nat(0u);
return x_1;
}
}
static lean_object* _init_l_AuthToken_refreshDuration() {
_start:
{
lean_object* x_1; 
x_1 = lean_unsigned_to_nat(3600u);
return x_1;
}
}
LEAN_EXPORT lean_object* l_AuthToken_refresh(lean_object* x_1, lean_object* x_2) {
_start:
{
uint8_t x_3; 
x_3 = !lean_is_exclusive(x_1);
if (x_3 == 0)
{
lean_object* x_4; lean_object* x_5; lean_object* x_6; lean_object* x_7; 
x_4 = lean_ctor_get(x_1, 2);
lean_dec(x_4);
x_5 = lean_ctor_get(x_1, 1);
lean_dec(x_5);
x_6 = l_AuthToken_refreshDuration;
x_7 = lean_nat_add(x_2, x_6);
lean_ctor_set(x_1, 2, x_2);
lean_ctor_set(x_1, 1, x_7);
return x_1;
}
else
{
lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; 
x_8 = lean_ctor_get(x_1, 0);
lean_inc(x_8);
lean_dec(x_1);
x_9 = l_AuthToken_refreshDuration;
x_10 = lean_nat_add(x_2, x_9);
x_11 = lean_alloc_ctor(0, 3, 0);
lean_ctor_set(x_11, 0, x_8);
lean_ctor_set(x_11, 1, x_10);
lean_ctor_set(x_11, 2, x_2);
return x_11;
}
}
}
lean_object* initialize_Init(uint8_t builtin, lean_object*);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_Ferro_AuthTokenProperties(uint8_t builtin, lean_object* w) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin, lean_io_mk_world());
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
l_AuthToken_currentTime = _init_l_AuthToken_currentTime();
lean_mark_persistent(l_AuthToken_currentTime);
l_AuthToken_refreshDuration = _init_l_AuthToken_refreshDuration();
lean_mark_persistent(l_AuthToken_refreshDuration);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
