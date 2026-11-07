use serde::{Deserialize, Serialize};

/// Who a flag is on for, besides everyone it is switched on for outright.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagTargets {
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    /// A stable share of all users, 0 to 100.
    #[serde(default)]
    pub percent: u8,
}

/// A boolean smbCloud Flag, as the owner API returns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flag {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub on: bool,
    #[serde(default)]
    pub targets: FlagTargets,
}

/// The fields to create or change. Fields left out are not sent, so an update
/// only touches what was given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FlagInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "FlagTargetsInput::is_empty")]
    pub targets: FlagTargetsInput,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FlagTargetsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emails: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

impl FlagTargetsInput {
    pub fn is_empty(&self) -> bool {
        self.emails.is_none() && self.domains.is_none() && self.percent.is_none()
    }
}

#[derive(Debug, Deserialize)]
pub struct FlagList {
    pub flags: Vec<Flag>,
}

#[derive(Debug, Deserialize)]
pub struct FlagEnvelope {
    pub flag: Flag,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_flag_reads_the_api_shape() {
        let flag: Flag = serde_json::from_value(json!({
            "key": "beta", "name": "Beta", "description": "", "on": true,
            "targets": { "emails": ["a@example.com"], "domains": [], "percent": 10 }
        }))
        .expect("flag");
        assert_eq!(flag.targets.percent, 10);
        assert_eq!(flag.targets.emails, vec!["a@example.com"]);
    }

    #[test]
    fn an_update_sends_only_what_was_given() {
        let input = FlagInput {
            on: Some(false),
            targets: FlagTargetsInput {
                percent: Some(25),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&input).expect("json"),
            json!({ "on": false, "targets": { "percent": 25 } })
        );
        assert_eq!(
            serde_json::to_value(FlagInput::default()).expect("json"),
            json!({})
        );
    }
}
