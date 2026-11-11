//! The iteration budget and the hold estimate (target-system-design §4.1, §4.5 ⟨2⟩;
//! economy.md §5 step 1; ADR-0008 §7).

use abw_ledger::policy::PricePolicy;

/// LLM calls that may offer tools (§4.1, `MAX_ITERATIONS = 5`).
pub const MAX_ITERATIONS: u32 = 5;
/// The closing turn after the iterations, which offers no tools (§4.1).
pub const FLUSH_TURNS: u32 = 1;

/// The hold for one run: `rate × max_tokens × (MAX_ITERATIONS + 1) + gas`, capped at the
/// price policy's `hold_cap_cu` (§4.5 ⟨2⟩). Saturating.
pub fn estimate_hold(price: &PricePolicy, max_tokens: u32) -> u64 {
    let rate = price
        .cu_per_input_token
        .saturating_add(price.cu_per_output_token);
    rate.saturating_mul(u64::from(max_tokens))
        .saturating_mul(u64::from(MAX_ITERATIONS + FLUSH_TURNS))
        .saturating_add(price.hold_gas_cu)
        .min(price.hold_cap_cu)
}
