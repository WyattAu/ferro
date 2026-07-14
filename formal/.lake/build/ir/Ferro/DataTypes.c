// Lean compiler output
// Module: Ferro.DataTypes
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
LEAN_EXPORT lean_object* l_instBEqContentHash;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7;
static lean_object* l_instBEqContentHash___closed__1;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11;
lean_object* l_String_quote(lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10;
uint8_t lean_string_dec_eq(lean_object*, lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12;
LEAN_EXPORT lean_object* l_instReprContentHash;
lean_object* lean_nat_to_int(lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3;
LEAN_EXPORT uint8_t l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52_(lean_object*, lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6;
static lean_object* l_instReprContentHash___closed__1;
lean_object* lean_string_length(lean_object*);
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____boxed(lean_object*, lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5;
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14_(lean_object*, lean_object*);
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8;
static lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1;
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52____boxed(lean_object*, lean_object*);
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1() {
_start:
{
lean_object* x_1; 
x_1 = lean_mk_string_unchecked("hash", 4, 4);
return x_1;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1;
x_2 = lean_alloc_ctor(3, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3() {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_box(0);
x_2 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2;
x_3 = lean_alloc_ctor(5, 2, 0);
lean_ctor_set(x_3, 0, x_1);
lean_ctor_set(x_3, 1, x_2);
return x_3;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4() {
_start:
{
lean_object* x_1; 
x_1 = lean_mk_string_unchecked(" := ", 4, 4);
return x_1;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4;
x_2 = lean_alloc_ctor(3, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6() {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3;
x_2 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5;
x_3 = lean_alloc_ctor(5, 2, 0);
lean_ctor_set(x_3, 0, x_1);
lean_ctor_set(x_3, 1, x_2);
return x_3;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(8u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8() {
_start:
{
lean_object* x_1; 
x_1 = lean_mk_string_unchecked("{ ", 2, 2);
return x_1;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8;
x_2 = lean_string_length(x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9;
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8;
x_2 = lean_alloc_ctor(3, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12() {
_start:
{
lean_object* x_1; 
x_1 = lean_mk_string_unchecked(" }", 2, 2);
return x_1;
}
}
static lean_object* _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12;
x_2 = lean_alloc_ctor(3, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14_(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; lean_object* x_4; lean_object* x_5; lean_object* x_6; uint8_t x_7; lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; lean_object* x_12; lean_object* x_13; lean_object* x_14; lean_object* x_15; lean_object* x_16; lean_object* x_17; 
x_3 = l_String_quote(x_1);
x_4 = lean_alloc_ctor(3, 1, 0);
lean_ctor_set(x_4, 0, x_3);
x_5 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7;
x_6 = lean_alloc_ctor(4, 2, 0);
lean_ctor_set(x_6, 0, x_5);
lean_ctor_set(x_6, 1, x_4);
x_7 = 0;
x_8 = lean_alloc_ctor(6, 1, 1);
lean_ctor_set(x_8, 0, x_6);
lean_ctor_set_uint8(x_8, sizeof(void*)*1, x_7);
x_9 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6;
x_10 = lean_alloc_ctor(5, 2, 0);
lean_ctor_set(x_10, 0, x_9);
lean_ctor_set(x_10, 1, x_8);
x_11 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11;
x_12 = lean_alloc_ctor(5, 2, 0);
lean_ctor_set(x_12, 0, x_11);
lean_ctor_set(x_12, 1, x_10);
x_13 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13;
x_14 = lean_alloc_ctor(5, 2, 0);
lean_ctor_set(x_14, 0, x_12);
lean_ctor_set(x_14, 1, x_13);
x_15 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10;
x_16 = lean_alloc_ctor(4, 2, 0);
lean_ctor_set(x_16, 0, x_15);
lean_ctor_set(x_16, 1, x_14);
x_17 = lean_alloc_ctor(6, 1, 1);
lean_ctor_set(x_17, 0, x_16);
lean_ctor_set_uint8(x_17, sizeof(void*)*1, x_7);
return x_17;
}
}
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14_(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
static lean_object* _init_l_instReprContentHash___closed__1() {
_start:
{
lean_object* x_1; 
x_1 = lean_alloc_closure((void*)(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____boxed), 2, 0);
return x_1;
}
}
static lean_object* _init_l_instReprContentHash() {
_start:
{
lean_object* x_1; 
x_1 = l_instReprContentHash___closed__1;
return x_1;
}
}
LEAN_EXPORT uint8_t l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52_(lean_object* x_1, lean_object* x_2) {
_start:
{
uint8_t x_3; 
x_3 = lean_string_dec_eq(x_1, x_2);
return x_3;
}
}
LEAN_EXPORT lean_object* l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52____boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
uint8_t x_3; lean_object* x_4; 
x_3 = l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52_(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
x_4 = lean_box(x_3);
return x_4;
}
}
static lean_object* _init_l_instBEqContentHash___closed__1() {
_start:
{
lean_object* x_1; 
x_1 = lean_alloc_closure((void*)(l___private_Ferro_DataTypes_0__beqContentHash____x40_Ferro_DataTypes___hyg_52____boxed), 2, 0);
return x_1;
}
}
static lean_object* _init_l_instBEqContentHash() {
_start:
{
lean_object* x_1; 
x_1 = l_instBEqContentHash___closed__1;
return x_1;
}
}
lean_object* initialize_Init(uint8_t builtin, lean_object*);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_Ferro_DataTypes(uint8_t builtin, lean_object* w) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin, lean_io_mk_world());
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__1);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__2);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__3);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__4);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__5);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__6);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__7);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__8);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__9);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__10);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__11);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__12);
l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13 = _init_l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13();
lean_mark_persistent(l___private_Ferro_DataTypes_0__reprContentHash____x40_Ferro_DataTypes___hyg_14____closed__13);
l_instReprContentHash___closed__1 = _init_l_instReprContentHash___closed__1();
lean_mark_persistent(l_instReprContentHash___closed__1);
l_instReprContentHash = _init_l_instReprContentHash();
lean_mark_persistent(l_instReprContentHash);
l_instBEqContentHash___closed__1 = _init_l_instBEqContentHash___closed__1();
lean_mark_persistent(l_instBEqContentHash___closed__1);
l_instBEqContentHash = _init_l_instBEqContentHash();
lean_mark_persistent(l_instBEqContentHash);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
