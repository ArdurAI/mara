//! M1-04: estimate `mara.cost_usd` from token usage + configured per-million rates.

use mara_core::Event;
use mara_core::config::GenAiPricingConfig;
use mara_schema::{CostConfidence, CostSource};

/// Populate `mara.cost_usd` / `mara.cost_source` / `mara.cost_confidence` from usage + pricing.
pub(crate) fn apply_gen_ai_cost_estimate(
    ev: &mut Event,
    pricing: &GenAiPricingConfig,
    request_truncated: bool,
    response_truncated: bool,
) {
    ev.mara.cost_source = Some(CostSource::MaraEstimated);

    if request_truncated || response_truncated {
        ev.mara.cost_confidence = Some(CostConfidence::Low);
    }

    if !pricing.estimate_enabled {
        ev.mara.cost_usd = Some(0.0);
        if ev.mara.cost_confidence.is_none() {
            ev.mara.cost_confidence = Some(CostConfidence::Low);
        }
        return;
    }

    let in_t = ev.gen_ai.usage.input_tokens.unwrap_or(0);
    let out_t = ev.gen_ai.usage.output_tokens.unwrap_or(0);
    if in_t == 0 && out_t == 0 {
        ev.mara.cost_usd = Some(0.0);
        if ev.mara.cost_confidence.is_none() {
            ev.mara.cost_confidence = Some(if request_truncated || response_truncated {
                CostConfidence::Low
            } else {
                CostConfidence::Medium
            });
        }
        return;
    }

    let model = ev
        .gen_ai
        .request
        .model
        .as_deref()
        .or(ev.gen_ai.response.model.as_deref())
        .unwrap_or("");

    let (in_rate, out_rate) = resolve_per_million_rates(model, pricing);
    let cost = (in_t as f64 / 1_000_000.0) * in_rate + (out_t as f64 / 1_000_000.0) * out_rate;
    ev.mara.cost_usd = Some(cost);

    if ev.mara.cost_confidence.is_none() {
        ev.mara.cost_confidence = Some(if request_truncated || response_truncated {
            CostConfidence::Low
        } else {
            CostConfidence::High
        });
    }
}

fn resolve_per_million_rates(model: &str, pricing: &GenAiPricingConfig) -> (f64, f64) {
    let mut best: Option<(usize, f64, f64)> = None;
    for row in &pricing.models {
        if model.starts_with(&row.prefix) {
            let len = row.prefix.len();
            if best.as_ref().is_none_or(|(bl, _, _)| len > *bl) {
                best = Some((len, row.input_per_million_usd, row.output_per_million_usd));
            }
        }
    }
    if let Some((_, in_r, out_r)) = best {
        (in_r, out_r)
    } else {
        (pricing.default_input_per_million_usd, pricing.default_output_per_million_usd)
    }
}

#[cfg(test)]
mod tests {
    use mara_core::config::{GenAiModelPriceRow, GenAiPricingConfig, ServerConfig};
    use mara_schema::{CostConfidence, CostSource, EventKind};

    use super::*;

    #[test]
    fn disabled_yields_zero() {
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.usage.input_tokens = Some(100);
        ev.gen_ai.usage.output_tokens = Some(50);
        let p = GenAiPricingConfig {
            estimate_enabled: false,
            ..Default::default()
        };
        apply_gen_ai_cost_estimate(&mut ev, &p, false, false);
        assert_eq!(ev.mara.cost_usd, Some(0.0));
        assert_eq!(ev.mara.cost_confidence, Some(CostConfidence::Low));
    }

    #[test]
    fn applies_defaults_per_million() {
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.usage.input_tokens = Some(1_000_000);
        ev.gen_ai.usage.output_tokens = Some(1_000_000);
        let p = GenAiPricingConfig {
            estimate_enabled: true,
            default_input_per_million_usd: 0.1,
            default_output_per_million_usd: 0.4,
            ..Default::default()
        };
        apply_gen_ai_cost_estimate(&mut ev, &p, false, false);
        assert!((ev.mara.cost_usd.expect("cost") - 0.5).abs() < 1e-9);
        assert_eq!(ev.mara.cost_confidence, Some(CostConfidence::High));
    }

    #[test]
    fn truncation_marks_low_confidence() {
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.usage.input_tokens = Some(1_000_000);
        ev.gen_ai.usage.output_tokens = Some(1);
        let p = GenAiPricingConfig {
            estimate_enabled: true,
            default_input_per_million_usd: 1.0,
            default_output_per_million_usd: 1.0,
            ..Default::default()
        };
        apply_gen_ai_cost_estimate(&mut ev, &p, true, false);
        assert_eq!(ev.mara.cost_confidence, Some(CostConfidence::Low));
    }

    #[test]
    fn longest_prefix_wins() {
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.request.model = Some("gpt-oss:120b".into());
        ev.gen_ai.usage.input_tokens = Some(2_000_000);
        ev.gen_ai.usage.output_tokens = Some(0);
        let p = GenAiPricingConfig {
            estimate_enabled: true,
            default_input_per_million_usd: 1.0,
            default_output_per_million_usd: 1.0,
            models: vec![
                GenAiModelPriceRow {
                    prefix: "gpt".into(),
                    input_per_million_usd: 0.5,
                    output_per_million_usd: 0.5,
                },
                GenAiModelPriceRow {
                    prefix: "gpt-oss".into(),
                    input_per_million_usd: 0.25,
                    output_per_million_usd: 0.25,
                },
            ],
        };
        apply_gen_ai_cost_estimate(&mut ev, &p, false, false);
        assert!((ev.mara.cost_usd.expect("cost") - 0.5).abs() < 1e-9);
        assert_eq!(ev.mara.cost_source, Some(CostSource::MaraEstimated));
    }

    #[test]
    fn server_defaults_merge() {
        let mut s = ServerConfig::default();
        s.gen_ai_pricing.estimate_enabled = true;
        s.gen_ai_pricing.default_input_per_million_usd = 2.0;
        s.gen_ai_pricing.default_output_per_million_usd = 3.0;
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.usage.input_tokens = Some(500_000);
        ev.gen_ai.usage.output_tokens = Some(500_000);
        apply_gen_ai_cost_estimate(&mut ev, &s.gen_ai_pricing, false, false);
        assert!((ev.mara.cost_usd.expect("cost") - 2.5).abs() < 1e-9);
    }

    #[test]
    fn charges_input_only_when_output_tokens_missing() {
        let mut ev = Event::now(EventKind::Completion, "t");
        ev.gen_ai.usage.input_tokens = Some(1_000_000);
        ev.gen_ai.usage.output_tokens = None;
        let p = GenAiPricingConfig {
            estimate_enabled: true,
            default_input_per_million_usd: 2.0,
            default_output_per_million_usd: 99.0,
            ..Default::default()
        };
        apply_gen_ai_cost_estimate(&mut ev, &p, false, false);
        assert!((ev.mara.cost_usd.expect("cost") - 2.0).abs() < 1e-9);
    }
}
