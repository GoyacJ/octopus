use chrono::Utc;
use harness_contracts::{ModelRef, PricingSnapshotId, UsageSnapshot};
use harness_model::*;
use rust_decimal::Decimal;

fn pricing(version: u32, billing_mode: BillingMode) -> ModelPricing {
    ModelPricing {
        pricing_id: "anthropic:sonnet".to_owned(),
        pricing_version: version,
        currency: Currency::Usd,
        input_per_million: Decimal::new(3, 0),
        output_per_million: Decimal::new(15, 0),
        cache_creation_per_million: Some(Decimal::new(4, 0)),
        cache_read_per_million: Some(Decimal::new(1, 0)),
        image_per_image: None,
        last_updated: Utc::now(),
        source: PricingSource::Hardcoded,
        billing_mode,
    }
}

fn snapshot(version: u32) -> PricingSnapshotId {
    PricingSnapshotId {
        pricing_id: "anthropic:sonnet".to_owned(),
        version,
    }
}

fn model_ref() -> ModelRef {
    ModelRef {
        provider_id: "anthropic".to_owned(),
        model_id: "claude-sonnet".to_owned(),
    }
}

#[test]
fn noop_and_missing_snapshot_return_none() {
    let usage = UsageSnapshot::default();
    let model_ref = model_ref();
    let calculator = PricingTableCostCalculator::new(vec![pricing(1, BillingMode::Standard)]);

    assert!(NoopCostCalculator
        .compute(&model_ref, Some(&snapshot(1)), &usage)
        .is_none());
    assert!(calculator.compute(&model_ref, None, &usage).is_none());
    assert!(calculator
        .compute(&model_ref, Some(&snapshot(2)), &usage)
        .is_none());
}

#[test]
fn standard_pricing_calculates_all_token_components() {
    let usage = UsageSnapshot {
        input_tokens: 1_000_000,
        output_tokens: 2_000_000,
        cache_write_tokens: 1_000_000,
        cache_read_tokens: 1_000_000,
        cost_micros: 0,
    };
    let calculator = PricingTableCostCalculator::new(vec![pricing(1, BillingMode::Standard)]);

    let cost = calculator
        .compute(&model_ref(), Some(&snapshot(1)), &usage)
        .unwrap();

    assert_eq!(cost.cents, 3_300 + 400 + 100);
    assert_eq!(cost.micro_cents, 0);
    assert_eq!(cost.currency, Currency::Usd);
    assert_eq!(cost.breakdown.input, Some(300));
    assert_eq!(cost.breakdown.output, Some(3_000));
    assert_eq!(cost.breakdown.cache_creation, Some(400));
    assert_eq!(cost.breakdown.cache_read, Some(100));
    assert_eq!(cost.pricing_snapshot_id, Some(snapshot(1)));
}

#[test]
fn cached_batched_and_tiered_modes_are_applied() {
    let model_ref = model_ref();

    let cached = PricingTableCostCalculator::new(vec![ModelPricing {
        cache_read_per_million: None,
        ..pricing(
            1,
            BillingMode::Cached {
                cache_read_discount: Ratio(0.1),
            },
        )
    }]);
    let cached_cost = cached
        .compute(
            &model_ref,
            Some(&snapshot(1)),
            &UsageSnapshot {
                cache_read_tokens: 1_000_000,
                ..UsageSnapshot::default()
            },
        )
        .unwrap();
    assert_eq!(cached_cost.breakdown.cache_read, Some(30));

    let batched = PricingTableCostCalculator::new(vec![pricing(
        2,
        BillingMode::Batched {
            discount: Ratio(0.5),
        },
    )]);
    let batched_cost = batched
        .compute(
            &model_ref,
            Some(&snapshot(2)),
            &UsageSnapshot {
                input_tokens: 1_000_000,
                output_tokens: 1_000_000,
                ..UsageSnapshot::default()
            },
        )
        .unwrap();
    assert_eq!(batched_cost.cents, 900);

    let tiered = PricingTableCostCalculator::new(vec![pricing(
        3,
        BillingMode::Tiered {
            thresholds: vec![(2_000_000, Decimal::new(2, 0))],
        },
    )]);
    let tiered_cost = tiered
        .compute(
            &model_ref,
            Some(&snapshot(3)),
            &UsageSnapshot {
                input_tokens: 2_000_000,
                ..UsageSnapshot::default()
            },
        )
        .unwrap();
    assert_eq!(tiered_cost.breakdown.input, Some(400));
}
