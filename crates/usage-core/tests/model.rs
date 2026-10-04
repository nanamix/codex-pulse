use serde_json::json;
use usage_core::parse_limits;
#[test]
fn reset_count_uses_summary_not_detail_length_and_distinguishes_unknown() {
    use usage_core::parse_reset_count;
    assert_eq!(
        parse_reset_count(&json!({"rateLimitResetCredits":{"availableCount":7,"credits":[{}]}})),
        Some(7)
    );
    assert_eq!(
        parse_reset_count(&json!({"rateLimitResetCredits":{"availableCount":0}})),
        Some(0)
    );
    for v in [
        json!({}),
        json!({"rateLimitResetCredits":null}),
        json!({"rateLimitResetCredits":{"availableCount":-1}}),
        json!({"rateLimitResetCredits":{"availableCount":"7"}}),
    ] {
        assert_eq!(parse_reset_count(&v), None);
    }
}
#[test]
fn reset_expirations_preserve_no_expiry_and_do_not_invent_missing_details() {
    use usage_core::parse_reset_expirations;
    assert_eq!(
        parse_reset_expirations(
            &json!({"rateLimitResetCredits":{"credits":[{"status":"available","expiresAt":1730947200},{"status":"available","expiresAt":null},{"status":"redeemed","expiresAt":123},{"status":"available"}]}})
        ),
        Some(vec![Some(1730947200), None])
    );
    assert_eq!(
        parse_reset_expirations(&json!({"rateLimitResetCredits":{"credits":null}})),
        None
    );
    assert_eq!(
        parse_reset_expirations(&json!({"rateLimitResetCredits":{"credits":[]}})),
        Some(vec![])
    );
}
#[test]
fn preserves_server_windows_and_remaining() {
    let v = json!({"rateLimits":{"limitId":"codex","primary":{"usedPercent":25.0,"windowDurationMins":15,"resetsAt":1730947200},"secondary":null}});
    let b = parse_limits(v).unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].primary.as_ref().unwrap().remaining_percent(), 75.0);
    assert_eq!(
        b[0].primary.as_ref().unwrap().window_duration_mins,
        Some(15)
    );
    assert!(b[0].secondary.is_none());
}
#[test]
fn preserves_multiple_buckets() {
    let b =
        parse_limits(json!({"rateLimitsByLimitId":{"b":{"primary":null},"a":{"secondary":null}}}))
            .unwrap();
    assert_eq!(
        b.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
}
#[test]
fn rejects_out_of_range_and_missing_usage() {
    for primary in [
        json!({"usedPercent":101}),
        json!({"usedPercent":-1}),
        json!({"resetsAt":123}),
    ] {
        assert!(parse_limits(json!({"rateLimits":{"primary":primary}})).is_err());
    }
}
#[test]
fn empty_multi_view_falls_back_without_inventing_data() {
    assert!(parse_limits(json!({})).is_err());
    let b = parse_limits(json!({"rateLimitsByLimitId":{},"rateLimits":{"primary":null}})).unwrap();
    assert_eq!(b.len(), 1);
    assert!(b[0].primary.is_none());
}
