use std::collections::HashMap;

use harness_contracts::{ModelRef, PricingSnapshotId, UsageSnapshot};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::{BillingMode, Currency, ModelPricing, Ratio};

pub trait CostCalculator: Send + Sync + 'static {
    fn calculator_id(&self) -> &str;

    fn compute(
        &self,
        model_ref: &ModelRef,
        pricing_snapshot_id: Option<&PricingSnapshotId>,
        usage: &UsageSnapshot,
    ) -> Option<Cost>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    pub cents: u64,
    pub micro_cents: u64,
    pub currency: Currency,
    pub breakdown: CostBreakdown,
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CostBreakdown {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cache_creation: Option<u64>,
    pub cache_read: Option<u64>,
    pub image: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopCostCalculator;

impl CostCalculator for NoopCostCalculator {
    fn calculator_id(&self) -> &str {
        "noop"
    }

    fn compute(
        &self,
        _model_ref: &ModelRef,
        _pricing_snapshot_id: Option<&PricingSnapshotId>,
        _usage: &UsageSnapshot,
    ) -> Option<Cost> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct PricingTableCostCalculator {
    pricing: HashMap<(String, u32), ModelPricing>,
}

impl PricingTableCostCalculator {
    pub fn new(pricing: Vec<ModelPricing>) -> Self {
        Self {
            pricing: pricing
                .into_iter()
                .map(|pricing| {
                    (
                        (pricing.pricing_id.clone(), pricing.pricing_version),
                        pricing,
                    )
                })
                .collect(),
        }
    }
}

impl CostCalculator for PricingTableCostCalculator {
    fn calculator_id(&self) -> &str {
        "pricing-table"
    }

    fn compute(
        &self,
        _model_ref: &ModelRef,
        pricing_snapshot_id: Option<&PricingSnapshotId>,
        usage: &UsageSnapshot,
    ) -> Option<Cost> {
        let snapshot = pricing_snapshot_id?;
        let pricing = self
            .pricing
            .get(&(snapshot.pricing_id.clone(), snapshot.version))?;

        let input = token_component(
            usage.input_tokens,
            input_rate(pricing, usage.input_tokens),
            pricing,
        );
        let output = token_component(usage.output_tokens, pricing.output_per_million, pricing);
        let cache_creation = token_component(
            usage.cache_write_tokens,
            pricing
                .cache_creation_per_million
                .unwrap_or(pricing.input_per_million),
            pricing,
        );
        let cache_read =
            token_component(usage.cache_read_tokens, cache_read_rate(pricing), pricing);
        let total = input.hundredths
            + output.hundredths
            + cache_creation.hundredths
            + cache_read.hundredths;

        Some(Cost {
            cents: total / 100,
            micro_cents: total % 100,
            currency: pricing.currency.clone(),
            breakdown: CostBreakdown {
                input: input.cents(),
                output: output.cents(),
                cache_creation: cache_creation.cents(),
                cache_read: cache_read.cents(),
                image: None,
            },
            pricing_snapshot_id: Some(snapshot.clone()),
        })
    }
}

#[derive(Default)]
struct ComponentCost {
    hundredths: u64,
}

impl ComponentCost {
    fn cents(&self) -> Option<u64> {
        (self.hundredths > 0).then_some(self.hundredths / 100)
    }
}

fn token_component(
    tokens: u64,
    rate_per_million: Decimal,
    pricing: &ModelPricing,
) -> ComponentCost {
    if tokens == 0 {
        return ComponentCost::default();
    }

    let rate = apply_batch_discount(rate_per_million, pricing);
    let hundredths = (Decimal::from(tokens) * rate * Decimal::from(10_000_u64)
        / Decimal::from(1_000_000_u64))
    .round()
    .to_u64()
    .unwrap_or(u64::MAX);

    ComponentCost { hundredths }
}

fn input_rate(pricing: &ModelPricing, input_tokens: u64) -> Decimal {
    match &pricing.billing_mode {
        BillingMode::Tiered { thresholds } => thresholds
            .iter()
            .filter(|(threshold, _)| input_tokens >= *threshold)
            .max_by_key(|(threshold, _)| *threshold)
            .map(|(_, rate)| *rate)
            .unwrap_or(pricing.input_per_million),
        _ => pricing.input_per_million,
    }
}

fn cache_read_rate(pricing: &ModelPricing) -> Decimal {
    match &pricing.billing_mode {
        BillingMode::Cached {
            cache_read_discount,
        } => pricing
            .cache_read_per_million
            .unwrap_or_else(|| pricing.input_per_million * ratio(*cache_read_discount)),
        _ => pricing
            .cache_read_per_million
            .unwrap_or(pricing.input_per_million),
    }
}

fn apply_batch_discount(rate: Decimal, pricing: &ModelPricing) -> Decimal {
    match pricing.billing_mode {
        BillingMode::Batched { discount } => rate * ratio(discount),
        _ => rate,
    }
}

fn ratio(value: Ratio) -> Decimal {
    Decimal::from_f32_retain(value.0).unwrap_or(Decimal::ZERO)
}
