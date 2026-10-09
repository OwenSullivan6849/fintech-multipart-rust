use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    pub payment_id: String,
    pub account_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub media_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskAction {
    AutoRelease,
    ManualReview,
}

pub fn decide(event: &PaymentEvent) -> RiskAction {
    if event.amount_minor <= 100_000 {
        RiskAction::AutoRelease
    } else {
        RiskAction::ManualReview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_value_payment_requires_review() {
        let event = PaymentEvent {
            payment_id: "pay_42".into(),
            account_id: "acct_7".into(),
            amount_minor: 100_001,
            currency: "USD".into(),
            media_key: "evidence/pay_42.bin".into(),
        };
        assert_eq!(decide(&event), RiskAction::ManualReview);
    }
}

