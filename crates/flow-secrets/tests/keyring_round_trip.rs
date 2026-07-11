use flow_secrets::{get_openai_api_key, store_openai_api_key};

#[test]
fn keyring_round_trip() {
    let test_key = "sk-test-flow-linux-keyring-roundtrip";
    store_openai_api_key(test_key).expect("store should succeed");
    let loaded = get_openai_api_key()
        .expect("get should not error")
        .expect("key should exist after store");
    assert_eq!(loaded, test_key);
    store_openai_api_key("").expect("clear should succeed");
}
