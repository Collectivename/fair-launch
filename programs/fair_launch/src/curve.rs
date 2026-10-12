//! Integer twin of `src/curve.ts`. Floor square-root is unique, so both
//! sides agree on every input that fits in range.

use crate::FairError;
use anchor_lang::prelude::*;
use std::cmp::Ordering;

pub const BAND_NUM: u64 = 10_001;
pub const BAND_DEN: u64 = 10_000;
pub const QUOTE_GOAL: u128 = 10_000_000_000;
pub const TOKEN_SUPPLY_RAW: u128 = 1_000_000_000_000_000;
pub const RAW_PRICE_DENOMINATOR: u64 = 1_000_000_000_000_000;
pub const MIN_SQRT_PRICE: u128 = 4_295_048_016;
pub const MAX_SQRT_PRICE: u128 = 79_226_673_521_066_979_257_578_248_091;
const SWAP_BUFFER_NUM: u128 = 125;
const SWAP_BUFFER_DEN: u128 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FairCurve {
    pub sqrt_start: u128,
    pub sqrt_end: u128,
    pub liquidity: u128,
    pub base_out: u128,
    pub quote_out: u128,
}

#[derive(Clone, Copy, Debug)]
struct U256 {
    hi: u128,
    lo: u128,
}

fn cmp(a: U256, b: U256) -> Ordering {
    a.hi.cmp(&b.hi).then(a.lo.cmp(&b.lo))
}

fn mul_u128(a: u128, b: u128) -> Result<U256> {
    let a0 = (a as u64) as u128;
    let a1 = a >> 64;
    let b0 = (b as u64) as u128;
    let b1 = b >> 64;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64)
        .checked_add(p01 & (u64::MAX as u128))
        .and_then(|v| v.checked_add(p10 & (u64::MAX as u128)))
        .ok_or_else(|| error!(FairError::Math))?;
    let lo = (p00 & (u64::MAX as u128)) | ((mid & (u64::MAX as u128)) << 64);
    let hi = p11
        .checked_add(p01 >> 64)
        .and_then(|v| v.checked_add(p10 >> 64))
        .and_then(|v| v.checked_add(mid >> 64))
        .ok_or_else(|| error!(FairError::Math))?;
    Ok(U256 { hi, lo })
}

fn mul_u256_u64(n: U256, m: u64) -> Result<U256> {
    let p_lo = mul_u128(n.lo, m as u128)?;
    let p_hi = mul_u128(n.hi, m as u128)?;
    if p_hi.hi != 0 {
        return err!(FairError::Math);
    }
    let hi = p_lo
        .hi
        .checked_add(p_hi.lo)
        .ok_or_else(|| error!(FairError::Math))?;
    Ok(U256 { lo: p_lo.lo, hi })
}

/// Little-endian limbs. `d` must be non-zero and fit in a u64, which both
/// `10^15` and `10_000` do.
fn div_u256_u64(n: U256, d: u64) -> Result<U256> {
    if d == 0 {
        return err!(FairError::Math);
    }
    let limbs = [
        n.lo as u64,
        (n.lo >> 64) as u64,
        n.hi as u64,
        (n.hi >> 64) as u64,
    ];
    let mut rem: u128 = 0;
    let mut q = [0u64; 4];
    for i in (0..4).rev() {
        rem = (rem << 64) | (limbs[i] as u128);
        q[i] = (rem / (d as u128)) as u64;
        rem %= d as u128;
    }
    Ok(U256 {
        lo: (q[0] as u128) | ((q[1] as u128) << 64),
        hi: (q[2] as u128) | ((q[3] as u128) << 64),
    })
}

fn isqrt(n: U256) -> Result<u128> {
    if n.hi == 0 && n.lo < 2 {
        return Ok(n.lo);
    }
    let mut lo = 0u128;
    let mut hi = if n.hi == 0 { 1u128 << 64 } else { u128::MAX };
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        match mul_u128(mid, mid) {
            Ok(sq) if cmp(sq, n) != Ordering::Greater => lo = mid,
            _ => hi = mid,
        }
    }
    if let Ok(sq) = mul_u128(hi, hi) {
        if cmp(sq, n) != Ordering::Greater {
            return Ok(hi);
        }
    }
    Ok(lo)
}

fn ceil_div(numer: U256, denom: u128) -> Result<u128> {
    if denom == 0 {
        return err!(FairError::Math);
    }
    if numer.hi == 0 && numer.lo == 0 {
        return Ok(0);
    }
    let mut lo = 0u128;
    let mut hi = u128::MAX;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        match mul_u128(mid, denom) {
            Ok(prod) if cmp(prod, numer) == Ordering::Less => lo = mid,
            _ => hi = mid,
        }
    }
    if let Ok(prod) = mul_u128(lo, denom) {
        if cmp(prod, numer) != Ordering::Less {
            return Ok(lo);
        }
    }
    match mul_u128(hi, denom) {
        Ok(prod) if cmp(prod, numer) != Ordering::Less => Ok(hi),
        _ => err!(FairError::BandTooTight),
    }
}

pub fn sqrt_price_from_market_cap(market_cap_lamports: u64) -> Result<u128> {
    if market_cap_lamports == 0 {
        return err!(FairError::PriceOutOfRange);
    }
    let rad = div_u256_u64(
        U256 {
            hi: market_cap_lamports as u128,
            lo: 0,
        },
        RAW_PRICE_DENOMINATOR,
    )?;
    let sqrt = isqrt(rad)?;
    if sqrt < MIN_SQRT_PRICE || sqrt >= MAX_SQRT_PRICE {
        return err!(FairError::PriceOutOfRange);
    }
    Ok(sqrt)
}

pub fn fair_curve(sqrt_start: u128) -> Result<FairCurve> {
    if sqrt_start < MIN_SQRT_PRICE || sqrt_start >= MAX_SQRT_PRICE {
        return err!(FairError::PriceOutOfRange);
    }
    let start_sq = mul_u128(sqrt_start, sqrt_start)?;
    let scaled = mul_u256_u64(start_sq, BAND_NUM)?;
    let end_rad = div_u256_u64(scaled, BAND_DEN)?;
    let mut sqrt_end = isqrt(end_rad)?;
    if sqrt_end <= sqrt_start {
        sqrt_end = sqrt_start
            .checked_add(1)
            .ok_or_else(|| error!(FairError::Math))?;
    }
    if sqrt_end > MAX_SQRT_PRICE {
        return err!(FairError::PriceOutOfRange);
    }
    let delta = sqrt_end - sqrt_start;
    if delta <= QUOTE_GOAL {
        return err!(FairError::BandTooTight);
    }
    let liquidity = ceil_div(
        U256 {
            hi: QUOTE_GOAL,
            lo: 0,
        },
        delta,
    )?;
    if liquidity == 0 {
        return err!(FairError::BandTooTight);
    }
    let product = mul_u128(liquidity, delta)?;
    let quote_out = product.hi;
    if quote_out < QUOTE_GOAL || quote_out > QUOTE_GOAL + 1 {
        return err!(FairError::Math);
    }
    let base_den = sqrt_start
        .checked_mul(sqrt_end)
        .ok_or_else(|| error!(FairError::Math))?;
    let base_out = ceil_div(product, base_den)?;
    let with_buffer = base_out
        .checked_mul(SWAP_BUFFER_NUM)
        .ok_or_else(|| error!(FairError::Math))?
        / SWAP_BUFFER_DEN;
    if with_buffer > TOKEN_SUPPLY_RAW {
        return err!(FairError::SupplyTooSmall);
    }
    Ok(FairCurve {
        sqrt_start,
        sqrt_end,
        liquidity,
        base_out,
        quote_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hundred_sol_matches_the_checked_band() {
        let sqrt = sqrt_price_from_market_cap(100_000_000_000).unwrap();
        assert_eq!(sqrt, 184_467_440_737_095_516);
        let curve = fair_curve(sqrt).unwrap();
        assert_eq!(curve.sqrt_end, 184_476_663_878_559_598);
        assert_eq!(
            curve.liquidity,
            368_944_104_615_669_073_733_645_728_484_461_547
        );
        assert_eq!(curve.quote_out, 10_000_000_000);
        let ratio_num = mul_u128(curve.sqrt_end, curve.sqrt_end).unwrap();
        let ratio_den = mul_u128(curve.sqrt_start, curve.sqrt_start).unwrap();
        let left = mul_u256_u64(ratio_num, BAND_DEN).unwrap();
        let right = mul_u256_u64(ratio_den, BAND_NUM).unwrap();
        assert!(cmp(left, right) != Ordering::Greater);
    }

    #[test]
    fn fifty_and_thousand_sol_collect_ten_sol() {
        for (mc, sqrt_start, sqrt_end, liquidity) in [
            (
                50_000_000_000u64,
                130_438_178_253_327_822u128,
                130_444_699_999_200_917u128,
                521_765_756_505_093_569_101_379_517_253_240_122u128,
            ),
            (
                1_000_000_000_000,
                583_337_266_871_351_588,
                583_366_433_005_560_028,
                116_670_369_987_692_325_434_668_775_803_996_598,
            ),
        ] {
            let sqrt = sqrt_price_from_market_cap(mc).unwrap();
            assert_eq!(sqrt, sqrt_start);
            let curve = fair_curve(sqrt).unwrap();
            assert_eq!(curve.sqrt_end, sqrt_end);
            assert_eq!(curve.liquidity, liquidity);
            assert_eq!(curve.quote_out, 10_000_000_000);
        }
    }

    #[test]
    fn one_sol_does_not_fit_the_band() {
        let sqrt = sqrt_price_from_market_cap(1_000_000_000).unwrap();
        assert!(fair_curve(sqrt).is_err());
    }
}
