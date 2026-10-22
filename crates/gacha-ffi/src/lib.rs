#![allow(unsafe_op_in_unsafe_fn)]

//! C ABI。指针由 [`gacha_free`] 归还；空指针可以传入 `gacha_free`。
//! 概率、计数和稀有度都是十进制字符串，不使用 `f64` 作为单步合同。
//! 分析报告里的多步质量才是 `f64`。

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_void;
use std::path::PathBuf;
use std::ptr;
use std::sync::{LazyLock, Mutex};

use gacha_engine::{Branch, Draw, FiredArg, HitQuery, HitReport, RuntimeError, Source, Target};

/// 调用方只传递指针，不读取字段。
#[repr(C)]
pub struct GachaEngine {
    _private: usize,
}

/// 调用方只传递指针，不读取字段。
#[repr(C)]
pub struct GachaSession {
    _private: usize,
}

#[repr(C)]
pub struct GachaSource {
    pub name: *const c_char,
    pub text: *const c_char,
}

#[repr(C)]
pub struct GachaError {
    pub code: *const c_char,
    pub message: *const c_char,
}

pub const GACHA_ARG_COUNT: i32 = 1;
pub const GACHA_ARG_BOOL: i32 = 2;
pub const GACHA_ARG_RARITY: i32 = 3;
pub const GACHA_ARG_ITEM: i32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaArg {
    pub kind: i32,
    pub text: *const c_char,
    pub bool_value: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaEvent {
    pub name: *const c_char,
    pub args: *const GachaArg,
    pub arg_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaBranch {
    pub entry: *const c_char,
    pub rarity: *const c_char,
    pub numer: *const c_char,
    pub denom: *const c_char,
    pub events: *const GachaEvent,
    pub event_count: usize,
}

#[repr(C)]
pub struct GachaDraw {
    pub guarantee: *const c_char,
    pub branches: *const GachaBranch,
    pub branch_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaActionInfo {
    pub name: *const c_char,
    pub min_times: u32,
    pub max_times: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaBannerInfo {
    pub name: *const c_char,
    pub actions: *const GachaActionInfo,
    pub action_count: usize,
}

#[repr(C)]
pub struct GachaBannerList {
    pub items: *const GachaBannerInfo,
    pub count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaStateInfo {
    pub name: *const c_char,
    pub scope: *const c_char,
    pub kind: *const c_char,
}

#[repr(C)]
pub struct GachaStateList {
    pub items: *const GachaStateInfo,
    pub count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaSlot {
    pub scope: *const c_char,
    pub group: *const c_char,
    pub name: *const c_char,
    pub kind: *const c_char,
    pub count_decimal: *const c_char,
    pub bool_value: i32,
}

#[repr(C)]
pub struct GachaSlotList {
    pub items: *const GachaSlot,
    pub count: usize,
}

pub const GACHA_TARGET_ITEM: i32 = 1;
pub const GACHA_TARGET_ANY: i32 = 2;
pub const GACHA_TARGET_RARITY: i32 = 3;
pub const GACHA_TARGET_TAG: i32 = 4;

#[repr(C)]
pub struct GachaHitQuery {
    pub banner: *const c_char,
    pub action: *const c_char,
    pub times: u32,
    pub actions: u32,
    pub copies: u32,
    pub target_kind: i32,
    pub target_item: *const c_char,
    pub target_items: *const *const c_char,
    pub target_item_count: usize,
    pub target_rarity: *const c_char,
    pub target_tag: *const c_char,
    pub quantiles: *const f64,
    pub quantile_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaHitPoint {
    pub index: u32,
    pub hit_probability: f64,
    pub cumulative_probability: f64,
    pub survival_probability: f64,
    pub surviving_states: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GachaQuantile {
    pub quantile: f64,
    pub index: u32,
    pub has_index: i32,
}

#[repr(C)]
pub struct GachaHitReport {
    pub draws: u32,
    pub actions: u32,
    pub times: u32,
    pub probability_within: f64,
    pub probability_not_hit: f64,
    pub expected_draw_if_hit: f64,
    pub has_expected_draw: i32,
    pub expected_draws_capped: f64,
    pub expected_action_if_hit: f64,
    pub has_expected_action: i32,
    pub expected_actions_capped: f64,
    pub peak_surviving_states: u64,
    pub final_surviving_states: u64,
    pub draws_distribution: *const GachaHitPoint,
    pub draw_count: usize,
    pub actions_distribution: *const GachaHitPoint,
    pub action_count: usize,
    pub draw_quantiles: *const GachaQuantile,
    pub draw_quantile_count: usize,
    pub action_quantiles: *const GachaQuantile,
    pub action_quantile_count: usize,
}

struct Record {
    drop_fn: unsafe fn(usize),
    data: usize,
}

static REGISTRY: LazyLock<Mutex<HashMap<usize, Record>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn remember(public: *mut u8, data: usize, drop_fn: unsafe fn(usize)) {
    let mut guard = REGISTRY.lock().unwrap_or_else(|err| err.into_inner());
    guard.insert(public as usize, Record { drop_fn, data });
}

unsafe fn drop_box<T>(data: usize) {
    drop(Box::from_raw(data as *mut T));
}

fn leak_box<T>(value: T) -> *mut T {
    let data = Box::into_raw(Box::new(value));
    remember(data as *mut u8, data as usize, drop_box::<T>);
    data
}

fn cstring(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap())
}

fn fail(error: *mut *mut GachaError, err: RuntimeError) -> i32 {
    if !error.is_null() {
        let code = cstring(err.code);
        let message = cstring(&err.message);
        let mut owned = ErrorOwned {
            view: GachaError {
                code: ptr::null(),
                message: ptr::null(),
            },
            code,
            message,
        };
        owned.view.code = owned.code.as_ptr();
        owned.view.message = owned.message.as_ptr();
        let leaked = leak_box(owned);
        unsafe { *error = leaked as *mut GachaError };
    }
    1
}

#[repr(C)]
struct ErrorOwned {
    view: GachaError,
    code: CString,
    message: CString,
}

unsafe fn text<'a>(ptr: *const c_char, what: &str) -> Result<&'a str, RuntimeError> {
    if ptr.is_null() {
        return Err(RuntimeError::new("E_RANGE", format!("{what} 为空")));
    }
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    std::str::from_utf8(bytes)
        .map_err(|_| RuntimeError::new("E_RANGE", format!("{what} 不是 UTF-8")))
}

fn parse_u128(text: &str) -> Result<u128, RuntimeError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RuntimeError::new(
            "E_RANGE",
            format!("不是十进制整数 {text}"),
        ));
    }
    text.parse()
        .map_err(|_| RuntimeError::new("E_RANGE", format!("整数超出范围 {text}")))
}

fn times_of(value: u128) -> Result<u32, RuntimeError> {
    u32::try_from(value).map_err(|_| RuntimeError::new("E_RANGE", "动作次数超出 32 位"))
}

fn output<T>(out: *mut *mut T, error: *mut *mut GachaError) -> Result<(), RuntimeError> {
    if out.is_null() {
        Err(RuntimeError::new("E_RANGE", "输出指针为空"))
    } else {
        if !error.is_null() {
            unsafe { *error = ptr::null_mut() };
        }
        Ok(())
    }
}

fn publish<T>(out: *mut *mut T, ptr: *mut T) -> i32 {
    unsafe { *out = ptr };
    0
}

/// # Safety
/// `ptr` 必须是空指针，或本库此前返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let record = {
        let mut guard = REGISTRY.lock().unwrap_or_else(|err| err.into_inner());
        guard.remove(&(ptr as usize))
    };
    if let Some(record) = record {
        unsafe { (record.drop_fn)(record.data) };
    }
}

/// # Safety
/// `sources` 在 `count > 0` 时必须指向 `count` 个以空字符结尾的 UTF-8 源。
/// 成功时 `*out` 由 [`gacha_free`] 释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_engine_compile(
    sources: *const GachaSource,
    count: usize,
    out: *mut *mut GachaEngine,
    error: *mut *mut GachaError,
) -> i32 {
    match compile_sources(sources, count) {
        Ok(engine) => match output(out, error) {
            Ok(()) => publish(out, leak_box(engine) as *mut GachaEngine),
            Err(err) => fail(error, err),
        },
        Err(err) => fail(error, err),
    }
}

unsafe fn compile_sources(
    sources: *const GachaSource,
    count: usize,
) -> Result<gacha_engine::Compiled, RuntimeError> {
    if count > 0 && sources.is_null() {
        return Err(RuntimeError::new("E_RANGE", "源文件指针为空"));
    }
    let slice = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(sources, count) }
    };
    let mut names = Vec::with_capacity(slice.len());
    let mut texts = Vec::with_capacity(slice.len());
    for source in slice {
        names.push(unsafe { text(source.name, "源文件名") }?.to_string());
        texts.push(unsafe { text(source.text, "源文件正文") }?.to_string());
    }
    let views: Vec<Source<'_>> = names
        .iter()
        .zip(texts.iter())
        .map(|(name, text)| Source { name, text })
        .collect();
    gacha_engine::compile(&views).map_err(RuntimeError::from)
}

/// # Safety
/// `paths` 在 `count > 0` 时必须指向 `count` 个路径字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_engine_load(
    paths: *const *const c_char,
    count: usize,
    out: *mut *mut GachaEngine,
    error: *mut *mut GachaError,
) -> i32 {
    let loaded = (|| {
        if count > 0 && paths.is_null() {
            return Err(RuntimeError::new("E_RANGE", "路径指针为空"));
        }
        let slice = if count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(paths, count) }
        };
        let mut files = Vec::with_capacity(slice.len());
        for path in slice {
            files.push(PathBuf::from(unsafe { text(*path, "路径") }?));
        }
        gacha_engine::load(&files)
    })();
    match loaded {
        Ok(engine) => match output(out, error) {
            Ok(()) => publish(out, leak_box(engine) as *mut GachaEngine),
            Err(err) => fail(error, err),
        },
        Err(err) => fail(error, err),
    }
}

unsafe fn open_engine<'a>(
    ptr: *mut GachaEngine,
) -> Result<&'a gacha_engine::Compiled, RuntimeError> {
    if ptr.is_null() {
        Err(RuntimeError::new("E_RANGE", "引擎为空"))
    } else {
        Ok(unsafe { &*(ptr as *mut gacha_engine::Compiled) })
    }
}

unsafe fn open_session<'a>(
    ptr: *mut GachaSession,
) -> Result<&'a mut gacha_engine::Session, RuntimeError> {
    if ptr.is_null() {
        Err(RuntimeError::new("E_RANGE", "会话为空"))
    } else {
        Ok(unsafe { &mut *(ptr as *mut gacha_engine::Session) })
    }
}

/// # Safety
/// `engine` 必须是尚未释放的编译结果。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_engine_session(
    engine: *mut GachaEngine,
    out: *mut *mut GachaSession,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let engine = unsafe { open_engine(engine)? };
        output(out, error)?;
        Ok(engine.session())
    })() {
        Ok(session) => publish(out, leak_box(session) as *mut GachaSession),
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `engine` 必须是尚未释放的编译结果。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_engine_banners(
    engine: *mut GachaEngine,
    out: *mut *mut GachaBannerList,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let engine = unsafe { open_engine(engine)? };
        output(out, error)?;
        let mut items = Vec::new();
        for banner in engine.banners() {
            let actions = engine.actions(banner)?;
            items.push(owned_banner(banner, &actions)?);
        }
        Ok(seal_banners(items))
    })() {
        Ok(list) => publish(out, leak_box(list) as *mut GachaBannerList),
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `engine` 必须是尚未释放的编译结果。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_engine_states(
    engine: *mut GachaEngine,
    banner: *const c_char,
    out: *mut *mut GachaStateList,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let engine = unsafe { open_engine(engine)? };
        let banner = unsafe { text(banner, "卡池") }?;
        output(out, error)?;
        let states = engine.states(banner)?;
        Ok(seal_states(states))
    })() {
        Ok(list) => publish(out, leak_box(list) as *mut GachaStateList),
        Err(err) => fail(error, err),
    }
}

fn owned_banner(
    name: &str,
    actions: &[gacha_engine::ActionInfo],
) -> Result<BannerOwned, RuntimeError> {
    let mut action_owned = Vec::new();
    for action in actions {
        action_owned.push(ActionOwned {
            name: cstring(&action.name),
            min_times: times_of(action.min_times)?,
            max_times: times_of(action.max_times)?,
            view: GachaActionInfo {
                name: ptr::null(),
                min_times: 0,
                max_times: 0,
            },
        });
    }
    for action in &mut action_owned {
        action.view = GachaActionInfo {
            name: action.name.as_ptr(),
            min_times: action.min_times,
            max_times: action.max_times,
        };
    }
    let views = action_owned.iter().map(|action| action.view).collect();
    Ok(BannerOwned {
        name: cstring(name),
        actions: action_owned,
        views,
        view: GachaBannerInfo {
            name: ptr::null(),
            actions: ptr::null(),
            action_count: 0,
        },
    })
}

#[repr(C)]
struct ActionOwned {
    name: CString,
    min_times: u32,
    max_times: u32,
    view: GachaActionInfo,
}

#[repr(C)]
struct BannerOwned {
    name: CString,
    actions: Vec<ActionOwned>,
    views: Vec<GachaActionInfo>,
    view: GachaBannerInfo,
}

#[repr(C)]
struct BannerListOwned {
    view: GachaBannerList,
    items: Vec<BannerOwned>,
    views: Vec<GachaBannerInfo>,
}

fn seal_banners(mut items: Vec<BannerOwned>) -> BannerListOwned {
    for item in &mut items {
        item.view.name = item.name.as_ptr();
        item.view.actions = item.views.as_ptr();
        item.view.action_count = item.views.len();
    }
    let views = items.iter().map(|item| item.view).collect::<Vec<_>>();
    let mut owned = BannerListOwned {
        view: GachaBannerList {
            items: ptr::null(),
            count: views.len(),
        },
        items,
        views,
    };
    owned.view.items = owned.views.as_ptr();
    owned
}

#[repr(C)]
struct StateOwned {
    name: CString,
    scope: CString,
    kind: CString,
    view: GachaStateInfo,
}

#[repr(C)]
struct StateListOwned {
    view: GachaStateList,
    items: Vec<StateOwned>,
    views: Vec<GachaStateInfo>,
}

fn seal_states(states: Vec<gacha_engine::StateInfo>) -> StateListOwned {
    let mut items = Vec::new();
    for state in states {
        let mut owned = StateOwned {
            name: cstring(&state.name),
            scope: cstring(&state.scope),
            kind: cstring(&state.kind),
            view: GachaStateInfo {
                name: ptr::null(),
                scope: ptr::null(),
                kind: ptr::null(),
            },
        };
        owned.view = GachaStateInfo {
            name: owned.name.as_ptr(),
            scope: owned.scope.as_ptr(),
            kind: owned.kind.as_ptr(),
        };
        items.push(owned);
    }
    let views = items.iter().map(|item| item.view).collect::<Vec<_>>();
    let mut owned = StateListOwned {
        view: GachaStateList {
            items: ptr::null(),
            count: views.len(),
        },
        items,
        views,
    };
    owned.view.items = owned.views.as_ptr();
    owned
}

fn count_text(
    session: *mut GachaSession,
    banner: *const c_char,
    name: *const c_char,
) -> Result<String, RuntimeError> {
    let session = unsafe { open_session(session)? };
    let banner = unsafe { text(banner, "卡池") }?;
    let name = unsafe { text(name, "状态") }?;
    Ok(session.get_count(banner, name)?.to_string())
}

/// # Safety
/// `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_get_count(
    session: *mut GachaSession,
    banner: *const c_char,
    name: *const c_char,
    out: *mut *mut c_char,
    error: *mut *mut GachaError,
) -> i32 {
    match count_text(session, banner, name) {
        Ok(value) => match output(out, error) {
            Ok(()) => {
                let owned = cstring(&value);
                let ptr = owned.as_ptr() as *mut c_char;
                remember(
                    ptr as *mut u8,
                    Box::into_raw(Box::new(owned)) as usize,
                    drop_box::<CString>,
                );
                publish(out, ptr)
            }
            Err(err) => fail(error, err),
        },
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `value` 必须是十进制整数字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_set_count(
    session: *mut GachaSession,
    banner: *const c_char,
    name: *const c_char,
    value: *const c_char,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let name = unsafe { text(name, "状态") }?;
        let value = parse_u128(unsafe { text(value, "计数") }?)?;
        session.set_count(banner, name, value)
    })() {
        Ok(()) => {
            if !error.is_null() {
                unsafe { *error = ptr::null_mut() };
            }
            0
        }
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `session` 必须是尚未释放的会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_get_bool(
    session: *mut GachaSession,
    banner: *const c_char,
    name: *const c_char,
    out: *mut i32,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        if out.is_null() {
            return Err(RuntimeError::new("E_RANGE", "输出指针为空"));
        }
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let name = unsafe { text(name, "状态") }?;
        let value = session.get_bool(banner, name)?;
        unsafe { *out = i32::from(value) };
        if !error.is_null() {
            unsafe { *error = ptr::null_mut() };
        }
        Ok(())
    })() {
        Ok(()) => 0,
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `session` 必须是尚未释放的会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_set_bool(
    session: *mut GachaSession,
    banner: *const c_char,
    name: *const c_char,
    value: i32,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let name = unsafe { text(name, "状态") }?;
        session.set_bool(banner, name, value != 0)
    })() {
        Ok(()) => {
            if !error.is_null() {
                unsafe { *error = ptr::null_mut() };
            }
            0
        }
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_snapshot(
    session: *mut GachaSession,
    out: *mut *mut GachaSlotList,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        output(out, error)?;
        Ok(seal_slots(session.snapshot()))
    })() {
        Ok(list) => publish(out, leak_box(list) as *mut GachaSlotList),
        Err(err) => fail(error, err),
    }
}

#[repr(C)]
struct SlotOwned {
    scope: CString,
    group: CString,
    name: CString,
    kind: CString,
    count_decimal: CString,
    view: GachaSlot,
}

#[repr(C)]
struct SlotListOwned {
    view: GachaSlotList,
    items: Vec<SlotOwned>,
    views: Vec<GachaSlot>,
}

fn seal_slots(slots: Vec<gacha_engine::SlotState>) -> SlotListOwned {
    let mut items = Vec::new();
    for slot in slots {
        let kind = if slot.count.is_some() {
            "count"
        } else {
            "bool"
        };
        let mut owned = SlotOwned {
            scope: cstring(&slot.scope),
            group: cstring(&slot.group),
            name: cstring(&slot.name),
            kind: cstring(kind),
            count_decimal: cstring(
                &slot
                    .count
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            view: GachaSlot {
                scope: ptr::null(),
                group: ptr::null(),
                name: ptr::null(),
                kind: ptr::null(),
                count_decimal: ptr::null(),
                bool_value: i32::from(slot.flag.unwrap_or(false)),
            },
        };
        owned.view.scope = owned.scope.as_ptr();
        owned.view.group = owned.group.as_ptr();
        owned.view.name = owned.name.as_ptr();
        owned.view.kind = owned.kind.as_ptr();
        owned.view.count_decimal = if slot.count.is_some() {
            owned.count_decimal.as_ptr()
        } else {
            ptr::null()
        };
        items.push(owned);
    }
    let views = items.iter().map(|item| item.view).collect::<Vec<_>>();
    let mut owned = SlotListOwned {
        view: GachaSlotList {
            items: ptr::null(),
            count: views.len(),
        },
        items,
        views,
    };
    owned.view.items = owned.views.as_ptr();
    owned
}

/// # Safety
/// `slots` 的字符串在调用期间必须有效，所有权仍属于调用方。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_restore(
    session: *mut GachaSession,
    slots: *const GachaSlot,
    count: usize,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        if count > 0 && slots.is_null() {
            return Err(RuntimeError::new("E_RANGE", "状态指针为空"));
        }
        let slice = if count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(slots, count) }
        };
        let mut parsed = Vec::with_capacity(slice.len());
        for slot in slice {
            let kind = unsafe { text(slot.kind, "状态类型") }?;
            let (count, flag) = if kind == "count" {
                (
                    Some(parse_u128(unsafe { text(slot.count_decimal, "计数") }?)?),
                    None,
                )
            } else if kind == "bool" {
                (None, Some(slot.bool_value != 0))
            } else {
                return Err(RuntimeError::new(
                    "E_STATE_TYPE",
                    format!("未知状态类型 {kind}"),
                ));
            };
            parsed.push(gacha_engine::SlotState {
                scope: unsafe { text(slot.scope, "状态域") }?.to_string(),
                group: unsafe { text(slot.group, "状态组") }?.to_string(),
                name: unsafe { text(slot.name, "状态") }?.to_string(),
                count,
                flag,
            });
        }
        session.restore(&parsed)
    })() {
        Ok(()) => {
            if !error.is_null() {
                unsafe { *error = ptr::null_mut() };
            }
            0
        }
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_draw(
    session: *mut GachaSession,
    banner: *const c_char,
    action: *const c_char,
    out: *mut *mut GachaDraw,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let action = unsafe { text(action, "动作") }?;
        output(out, error)?;
        Ok(seal_draw(session.draw(banner, action)?))
    })() {
        Ok(draw) => publish(out, leak_box(draw) as *mut GachaDraw),
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `entry` 必须是本次抽取的条目名。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_apply(
    session: *mut GachaSession,
    banner: *const c_char,
    action: *const c_char,
    entry: *const c_char,
    out: *mut *mut GachaBranch,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let action = unsafe { text(action, "动作") }?;
        let entry = unsafe { text(entry, "条目") }?;
        output(out, error)?;
        Ok(seal_branch(session.apply(banner, action, entry)?))
    })() {
        Ok(branch) => publish(out, leak_box(branch) as *mut GachaBranch),
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `session` 必须是尚未释放的会话。`seed` 只决定这一次无偏抽样。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_sample(
    session: *mut GachaSession,
    banner: *const c_char,
    action: *const c_char,
    seed: u64,
    out: *mut *mut GachaBranch,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        let banner = unsafe { text(banner, "卡池") }?;
        let action = unsafe { text(action, "动作") }?;
        output(out, error)?;
        Ok(seal_branch(session.sample_with_seed(banner, action, seed)?))
    })() {
        Ok(branch) => publish(out, leak_box(branch) as *mut GachaBranch),
        Err(err) => fail(error, err),
    }
}

/// # Safety
/// `query` 的字符串和分位数组在调用期间必须有效。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gacha_session_analyze(
    session: *mut GachaSession,
    query: *const GachaHitQuery,
    out: *mut *mut GachaHitReport,
    error: *mut *mut GachaError,
) -> i32 {
    match (|| {
        let session = unsafe { open_session(session)? };
        if query.is_null() {
            return Err(RuntimeError::new("E_RANGE", "分析请求为空"));
        }
        let query = unsafe { &*query };
        output(out, error)?;
        let parsed = unsafe { parse_query(query)? };
        Ok(seal_report(session.analyze(&parsed)?))
    })() {
        Ok(report) => publish(out, leak_box(report) as *mut GachaHitReport),
        Err(err) => fail(error, err),
    }
}

unsafe fn parse_query(query: &GachaHitQuery) -> Result<HitQuery, RuntimeError> {
    let quantiles = if query.quantile_count == 0 {
        Vec::new()
    } else if query.quantiles.is_null() {
        return Err(RuntimeError::new("E_RANGE", "分位数指针为空"));
    } else {
        unsafe { std::slice::from_raw_parts(query.quantiles, query.quantile_count) }.to_vec()
    };
    let target = match query.target_kind {
        GACHA_TARGET_ITEM => Target::Item(text(query.target_item, "条目")?.to_string()),
        GACHA_TARGET_ANY => {
            if query.target_item_count > 0 && query.target_items.is_null() {
                return Err(RuntimeError::new("E_RANGE", "条目集合指针为空"));
            }
            let ptrs = if query.target_item_count == 0 {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(query.target_items, query.target_item_count) }
            };
            let mut ids = Vec::with_capacity(ptrs.len());
            for ptr in ptrs {
                ids.push(text(*ptr, "条目")?.to_string());
            }
            Target::Any(ids)
        }
        GACHA_TARGET_RARITY => {
            Target::RarityAtLeast(parse_u128(text(query.target_rarity, "稀有度")?)?)
        }
        GACHA_TARGET_TAG => Target::Tag(text(query.target_tag, "标签")?.to_string()),
        _ => return Err(RuntimeError::new("E_TARGET", "未知目标类型")),
    };
    Ok(HitQuery {
        banner: text(query.banner, "卡池")?.to_string(),
        action: text(query.action, "动作")?.to_string(),
        times: query.times,
        actions: query.actions,
        copies: query.copies,
        target,
        quantiles,
    })
}

#[repr(C)]
struct ArgOwned {
    text: CString,
    view: GachaArg,
}

#[repr(C)]
struct EventOwned {
    name: CString,
    args: Vec<ArgOwned>,
    views: Vec<GachaArg>,
    view: GachaEvent,
}

#[repr(C)]
struct BranchOwned {
    view: GachaBranch,
    entry: CString,
    rarity: CString,
    numer: CString,
    denom: CString,
    events: Vec<EventOwned>,
    event_views: Vec<GachaEvent>,
}

#[repr(C)]
struct DrawOwned {
    view: GachaDraw,
    guarantee: Option<CString>,
    branches: Vec<BranchOwned>,
    branch_views: Vec<GachaBranch>,
}

fn seal_branch(branch: Branch) -> BranchOwned {
    let mut events = Vec::new();
    for event in branch.events {
        let mut args = Vec::new();
        for arg in event.args {
            let (kind, text, bool_value) = match arg {
                FiredArg::Count(value) => (GACHA_ARG_COUNT, value.to_string(), 0),
                FiredArg::Bool(value) => (GACHA_ARG_BOOL, String::new(), i32::from(value)),
                FiredArg::Rarity(value) => (GACHA_ARG_RARITY, value.to_string(), 0),
                FiredArg::Item(value) => (GACHA_ARG_ITEM, value, 0),
            };
            let text = cstring(&text);
            let view = GachaArg {
                kind,
                text: text.as_ptr(),
                bool_value,
            };
            args.push(ArgOwned { text, view });
        }
        let views = args.iter().map(|arg| arg.view).collect::<Vec<_>>();
        let name = cstring(&event.name);
        let mut owned = EventOwned {
            name,
            args,
            views,
            view: GachaEvent {
                name: ptr::null(),
                args: ptr::null(),
                arg_count: 0,
            },
        };
        owned.view.name = owned.name.as_ptr();
        owned.view.args = owned.views.as_ptr();
        owned.view.arg_count = owned.views.len();
        events.push(owned);
    }
    let event_views = events.iter().map(|event| event.view).collect::<Vec<_>>();
    let entry = cstring(&branch.entry);
    let rarity = cstring(&branch.rarity.to_string());
    let numer = cstring(&branch.probability.numer().to_string());
    let denom = cstring(&branch.probability.denom().to_string());
    let mut owned = BranchOwned {
        entry,
        rarity,
        numer,
        denom,
        events,
        event_views,
        view: GachaBranch {
            entry: ptr::null(),
            rarity: ptr::null(),
            numer: ptr::null(),
            denom: ptr::null(),
            events: ptr::null(),
            event_count: 0,
        },
    };
    owned.view.entry = owned.entry.as_ptr();
    owned.view.rarity = owned.rarity.as_ptr();
    owned.view.numer = owned.numer.as_ptr();
    owned.view.denom = owned.denom.as_ptr();
    owned.view.events = owned.event_views.as_ptr();
    owned.view.event_count = owned.event_views.len();
    owned
}

fn seal_draw(draw: Draw) -> DrawOwned {
    let branches = draw
        .branches
        .into_iter()
        .map(seal_branch)
        .collect::<Vec<_>>();
    let branch_views = branches
        .iter()
        .map(|branch| branch.view)
        .collect::<Vec<_>>();
    let guarantee = draw.guarantee.as_deref().map(cstring);
    let mut owned = DrawOwned {
        view: GachaDraw {
            guarantee: ptr::null(),
            branches: ptr::null(),
            branch_count: branch_views.len(),
        },
        guarantee,
        branches,
        branch_views,
    };
    owned.view.guarantee = owned
        .guarantee
        .as_ref()
        .map(|text| text.as_ptr())
        .unwrap_or(ptr::null());
    owned.view.branches = owned.branch_views.as_ptr();
    owned
}

#[repr(C)]
struct ReportOwned {
    view: GachaHitReport,
    draws: Vec<GachaHitPoint>,
    actions: Vec<GachaHitPoint>,
    draw_quantiles: Vec<GachaQuantile>,
    action_quantiles: Vec<GachaQuantile>,
}

fn seal_report(report: HitReport) -> ReportOwned {
    let draws = report
        .draws_distribution
        .iter()
        .map(point_of)
        .collect::<Vec<_>>();
    let actions = report
        .actions_distribution
        .iter()
        .map(point_of)
        .collect::<Vec<_>>();
    let draw_quantiles = report
        .draw_quantiles
        .iter()
        .map(quantile_of)
        .collect::<Vec<_>>();
    let action_quantiles = report
        .action_quantiles
        .iter()
        .map(quantile_of)
        .collect::<Vec<_>>();
    let (expected_draw_if_hit, has_expected_draw) = flag_f64(report.expected_draw_if_hit);
    let (expected_action_if_hit, has_expected_action) = flag_f64(report.expected_action_if_hit);
    let mut owned = ReportOwned {
        view: GachaHitReport {
            draws: report.draws,
            actions: report.actions,
            times: report.times,
            probability_within: report.probability_within,
            probability_not_hit: report.probability_not_hit,
            expected_draw_if_hit,
            has_expected_draw,
            expected_draws_capped: report.expected_draws_capped,
            expected_action_if_hit,
            has_expected_action,
            expected_actions_capped: report.expected_actions_capped,
            peak_surviving_states: report.peak_surviving_states,
            final_surviving_states: report.final_surviving_states,
            draws_distribution: ptr::null(),
            draw_count: draws.len(),
            actions_distribution: ptr::null(),
            action_count: actions.len(),
            draw_quantiles: ptr::null(),
            draw_quantile_count: draw_quantiles.len(),
            action_quantiles: ptr::null(),
            action_quantile_count: action_quantiles.len(),
        },
        draws,
        actions,
        draw_quantiles,
        action_quantiles,
    };
    owned.view.draws_distribution = owned.draws.as_ptr();
    owned.view.actions_distribution = owned.actions.as_ptr();
    owned.view.draw_quantiles = owned.draw_quantiles.as_ptr();
    owned.view.action_quantiles = owned.action_quantiles.as_ptr();
    owned
}

fn point_of(point: &gacha_engine::HitPoint) -> GachaHitPoint {
    GachaHitPoint {
        index: point.index,
        hit_probability: point.hit_probability,
        cumulative_probability: point.cumulative_probability,
        survival_probability: point.survival_probability,
        surviving_states: point.surviving_states,
    }
}

fn quantile_of(quantile: &gacha_engine::Quantile) -> GachaQuantile {
    GachaQuantile {
        quantile: quantile.quantile,
        index: quantile.index.unwrap_or(0),
        has_index: i32::from(quantile.index.is_some()),
    }
}

fn flag_f64(value: Option<f64>) -> (f64, i32) {
    match value {
        Some(value) => (value, 1),
        None => (0.0, 0),
    }
}

#[cfg(test)]
mod abi_test {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn positive() -> *mut GachaEngine {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/gacharule/positive");
        let names = [
            "endfield.gmr",
            "genshin.gmr",
            "weapon_ten.gmr",
            "sample.gml",
            "banners.gmb",
        ];
        let texts: Vec<CString> = names
            .iter()
            .map(|name| cstring(&fs::read_to_string(base.join(name)).unwrap()))
            .collect();
        let labels: Vec<CString> = names.iter().map(|name| cstring(name)).collect();
        let sources: Vec<GachaSource> = labels
            .iter()
            .zip(texts.iter())
            .map(|(name, text)| GachaSource {
                name: name.as_ptr(),
                text: text.as_ptr(),
            })
            .collect();
        let mut engine = ptr::null_mut();
        let mut error = ptr::null_mut();
        let status = unsafe {
            gacha_engine_compile(sources.as_ptr(), sources.len(), &mut engine, &mut error)
        };
        assert_eq!(status, 0, "{}", unsafe { error_string(error) });
        engine
    }

    unsafe fn error_string(error: *mut GachaError) -> String {
        if error.is_null() {
            return String::new();
        }
        format!(
            "{} {}",
            CStr::from_ptr((*error).code).to_string_lossy(),
            CStr::from_ptr((*error).message).to_string_lossy()
        )
    }

    fn c(text: &str) -> CString {
        cstring(text)
    }

    fn rarity_sum(draw: *mut GachaDraw, rarity: &str) -> (u128, u128) {
        let draw = unsafe { &*draw };
        let branches = unsafe { std::slice::from_raw_parts(draw.branches, draw.branch_count) };
        let mut numer = 0u128;
        let mut denom = 1u128;
        for branch in branches {
            let got = unsafe { CStr::from_ptr(branch.rarity) }.to_string_lossy();
            if got != rarity {
                continue;
            }
            let left = unsafe { CStr::from_ptr(branch.numer) }
                .to_string_lossy()
                .parse::<u128>()
                .unwrap();
            let right = unsafe { CStr::from_ptr(branch.denom) }
                .to_string_lossy()
                .parse::<u128>()
                .unwrap();
            numer = numer * right + left * denom;
            denom *= right;
            let gcd = gcd(numer, denom);
            numer /= gcd;
            denom /= gcd;
        }
        (numer, denom)
    }

    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            let next = a % b;
            a = b;
            b = next;
        }
        a
    }

    #[test]
    fn c_abi_matches_positive_contract() {
        let engine = positive();
        let mut session = ptr::null_mut();
        let mut error = ptr::null_mut();
        assert_eq!(
            unsafe { gacha_engine_session(engine, &mut session, &mut error) },
            0
        );
        let banner = c("endfield_event_a");
        let action = c("draw");
        let pity = c("pity6");
        let value = c("65");
        assert_eq!(
            unsafe {
                gacha_session_set_count(
                    session,
                    banner.as_ptr(),
                    pity.as_ptr(),
                    value.as_ptr(),
                    &mut error,
                )
            },
            0
        );
        let mut draw = ptr::null_mut();
        assert_eq!(
            unsafe {
                gacha_session_draw(
                    session,
                    banner.as_ptr(),
                    action.as_ptr(),
                    &mut draw,
                    &mut error,
                )
            },
            0
        );
        assert_eq!(rarity_sum(draw, "6"), (29, 500));

        let mut report = ptr::null_mut();
        let kind_rarity = c("6");
        let query = GachaHitQuery {
            banner: banner.as_ptr(),
            action: action.as_ptr(),
            times: 1,
            actions: 1,
            copies: 1,
            target_kind: GACHA_TARGET_RARITY,
            target_item: ptr::null(),
            target_items: ptr::null(),
            target_item_count: 0,
            target_rarity: kind_rarity.as_ptr(),
            target_tag: ptr::null(),
            quantiles: ptr::null(),
            quantile_count: 0,
        };
        assert_eq!(
            unsafe { gacha_session_analyze(session, &query, &mut report, &mut error) },
            0
        );
        let probability = unsafe { (*report).probability_within };
        assert!((probability - 29.0 / 500.0).abs() < 1e-9);

        let genshin = c("genshin_event_a");
        let mut genshin_draw = ptr::null_mut();
        assert_eq!(
            unsafe {
                gacha_session_draw(
                    session,
                    genshin.as_ptr(),
                    action.as_ptr(),
                    &mut genshin_draw,
                    &mut error,
                )
            },
            0
        );
        assert_eq!(rarity_sum(genshin_draw, "5"), (3, 500));
        assert_eq!(rarity_sum(genshin_draw, "4"), (51, 1000));

        unsafe {
            gacha_free(draw as *mut c_void);
            gacha_free(report as *mut c_void);
            gacha_free(genshin_draw as *mut c_void);
            gacha_free(session as *mut c_void);
            gacha_free(engine as *mut c_void);
        }
    }
}
