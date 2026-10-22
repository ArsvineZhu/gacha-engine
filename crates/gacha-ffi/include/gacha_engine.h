#ifndef GACHA_ENGINE_H
#define GACHA_ENGINE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#define GACHA_ARG_COUNT 1

#define GACHA_ARG_BOOL 2

#define GACHA_ARG_RARITY 3

#define GACHA_ARG_ITEM 4

#define GACHA_TARGET_ITEM 1

#define GACHA_TARGET_ANY 2

#define GACHA_TARGET_RARITY 3

#define GACHA_TARGET_TAG 4

typedef struct GachaSource {
    const char *name;
    const char *text;
} GachaSource;

/**
 * 调用方只传递指针，不读取字段。
 */
typedef struct GachaEngine {
    size_t _private;
} GachaEngine;

typedef struct GachaError {
    const char *code;
    const char *message;
} GachaError;

/**
 * 调用方只传递指针，不读取字段。
 */
typedef struct GachaSession {
    size_t _private;
} GachaSession;

typedef struct GachaActionInfo {
    const char *name;
    uint32_t min_times;
    uint32_t max_times;
} GachaActionInfo;

typedef struct GachaBannerInfo {
    const char *name;
    const struct GachaActionInfo *actions;
    size_t action_count;
} GachaBannerInfo;

typedef struct GachaBannerList {
    const struct GachaBannerInfo *items;
    size_t count;
} GachaBannerList;

typedef struct GachaStateInfo {
    const char *name;
    const char *scope;
    const char *kind;
} GachaStateInfo;

typedef struct GachaStateList {
    const struct GachaStateInfo *items;
    size_t count;
} GachaStateList;

typedef struct GachaSlot {
    const char *scope;
    const char *group;
    const char *name;
    const char *kind;
    const char *count_decimal;
    int32_t bool_value;
} GachaSlot;

typedef struct GachaSlotList {
    const struct GachaSlot *items;
    size_t count;
} GachaSlotList;

typedef struct GachaArg {
    int32_t kind;
    const char *text;
    int32_t bool_value;
} GachaArg;

typedef struct GachaEvent {
    const char *name;
    const struct GachaArg *args;
    size_t arg_count;
} GachaEvent;

typedef struct GachaBranch {
    const char *entry;
    const char *rarity;
    const char *numer;
    const char *denom;
    const struct GachaEvent *events;
    size_t event_count;
} GachaBranch;

typedef struct GachaDraw {
    const char *guarantee;
    const struct GachaBranch *branches;
    size_t branch_count;
} GachaDraw;

typedef struct GachaHitQuery {
    const char *action;
    uint32_t times;
    uint32_t actions;
    uint32_t copies;
    int32_t target_kind;
    const char *target_item;
    const char *const *target_items;
    size_t target_item_count;
    const char *target_rarity;
    const char *target_tag;
    const double *quantiles;
    size_t quantile_count;
} GachaHitQuery;

typedef struct GachaHitPoint {
    uint32_t index;
    double hit_probability;
    double cumulative_probability;
    double survival_probability;
    uint64_t surviving_states;
} GachaHitPoint;

typedef struct GachaQuantile {
    double quantile;
    uint32_t index;
    int32_t has_index;
} GachaQuantile;

typedef struct GachaHitReport {
    uint32_t draws;
    uint32_t actions;
    uint32_t times;
    double probability_within;
    double probability_not_hit;
    double expected_draw_if_hit;
    int32_t has_expected_draw;
    double expected_draws_capped;
    double expected_action_if_hit;
    int32_t has_expected_action;
    double expected_actions_capped;
    uint64_t peak_surviving_states;
    uint64_t final_surviving_states;
    const struct GachaHitPoint *draws_distribution;
    size_t draw_count;
    const struct GachaHitPoint *actions_distribution;
    size_t action_count;
    const struct GachaQuantile *draw_quantiles;
    size_t draw_quantile_count;
    const struct GachaQuantile *action_quantiles;
    size_t action_quantile_count;
} GachaHitReport;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * # Safety
 * `ptr` 必须是空指针，或本库此前返回、尚未释放的指针。
 */
void gacha_free(void *ptr);

/**
 * # Safety
 * `sources` 在 `count > 0` 时必须指向 `count` 个以空字符结尾的 UTF-8 源。
 * 成功时 `*out` 由 [`gacha_free`] 释放。
 */
int32_t gacha_engine_compile(const struct GachaSource *sources,
                             size_t count,
                             struct GachaEngine **out,
                             struct GachaError **error);

/**
 * # Safety
 * `paths` 在 `count > 0` 时必须指向 `count` 个路径字符串。
 */
int32_t gacha_engine_load(const char *const *paths,
                          size_t count,
                          struct GachaEngine **out,
                          struct GachaError **error);

/**
 * # Safety
 * `engine` 必须是尚未释放的编译结果。
 */
int32_t gacha_engine_session(struct GachaEngine *engine,
                             const char *banner,
                             struct GachaSession **out,
                             struct GachaError **error);

/**
 * # Safety
 * `engine` 必须是尚未释放的编译结果。
 */
int32_t gacha_engine_banners(struct GachaEngine *engine,
                             struct GachaBannerList **out,
                             struct GachaError **error);

/**
 * # Safety
 * `engine` 必须是尚未释放的编译结果。
 */
int32_t gacha_engine_states(struct GachaEngine *engine,
                            const char *banner,
                            struct GachaStateList **out,
                            struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
 */
int32_t gacha_session_get_count(struct GachaSession *session,
                                const char *name,
                                char **out,
                                struct GachaError **error);

/**
 * # Safety
 * `value` 必须是十进制整数字符串。
 */
int32_t gacha_session_set_count(struct GachaSession *session,
                                const char *name,
                                const char *value,
                                struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。
 */
int32_t gacha_session_get_bool(struct GachaSession *session,
                               const char *name,
                               int32_t *out,
                               struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。
 */
int32_t gacha_session_set_bool(struct GachaSession *session,
                               const char *name,
                               int32_t value,
                               struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
 */
int32_t gacha_session_snapshot(struct GachaSession *session,
                               struct GachaSlotList **out,
                               struct GachaError **error);

/**
 * # Safety
 * `slots` 的字符串在调用期间必须有效，所有权仍属于调用方。
 */
int32_t gacha_session_restore(struct GachaSession *session,
                              const struct GachaSlot *slots,
                              size_t count,
                              struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。成功时 `*out` 由 [`gacha_free`] 释放。
 */
int32_t gacha_session_draw(struct GachaSession *session,
                           const char *action,
                           struct GachaDraw **out,
                           struct GachaError **error);

/**
 * # Safety
 * `entry` 必须是本次抽取的条目名。
 */
int32_t gacha_session_apply(struct GachaSession *session,
                            const char *action,
                            const char *entry,
                            struct GachaBranch **out,
                            struct GachaError **error);

/**
 * # Safety
 * `session` 必须是尚未释放的会话。`seed` 只决定这一次无偏抽样。
 */
int32_t gacha_session_sample(struct GachaSession *session,
                             const char *action,
                             uint64_t seed,
                             struct GachaBranch **out,
                             struct GachaError **error);

/**
 * # Safety
 * `query` 的字符串和分位数组在调用期间必须有效。
 */
int32_t gacha_session_analyze(struct GachaSession *session,
                              const struct GachaHitQuery *query,
                              struct GachaHitReport **out,
                              struct GachaError **error);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* GACHA_ENGINE_H */
