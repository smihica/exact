/* exact_content_regions.h — the content-region family's C ABI (LLP 1047.001
 * D4, D10, §9.11): a registered region's text requests, completions and
 * fallback. Its exports are `exact_apple::content_region_exports!`'s; the
 * family's Swift, and only it, calls them. */
#ifndef EXACT_CONTENT_REGIONS_H
#define EXACT_CONTENT_REGIONS_H

#include "exact.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Region completion takes one retained native artifact on every return path. */
typedef void (*ExactRegionReleaseFn)(void *owner);
uint32_t exact_region_request(ExactRuntime rt, uint64_t request, uint64_t known_source);
uint32_t exact_region_complete(ExactRuntime rt, uint64_t request, ExactMetrics metrics, void *owner, ExactRegionReleaseFn release);
/* A current incomplete flow returns the registration to native layout. */
uint32_t exact_region_fallback(ExactRuntime rt, uint64_t request);

#ifdef __cplusplus
}
#endif

#endif
