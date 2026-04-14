use manganis::{asset, option_asset, Asset};

#[test]
fn resolves_existing_asset() {
    const REQUIRED: Asset = asset!("/assets/asset.txt");
    const OPTIONAL: Option<Asset> = option_asset!("/assets/asset.txt");

    let optional = OPTIONAL.expect("option_asset! should return Some for existing assets");
    assert_eq!(optional.to_string(), REQUIRED.to_string());
}

#[test]
fn missing_asset_returns_none() {
    const OPTIONAL: Option<Asset> = option_asset!("/assets/does_not_exist.txt");

    assert!(OPTIONAL.is_none());
}
