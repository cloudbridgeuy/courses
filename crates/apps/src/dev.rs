use aws_sdk_dynamodb::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType,
};

// ---------------------------------------------------------------------------
// Local-dev helpers (never called by the production server)
// ---------------------------------------------------------------------------

/// Builds a DynamoDB client pointed at a local endpoint (e.g. DynamoDB Local)
/// with static dummy credentials. For local development only — never used by
/// the server, which resolves real configuration via the standard AWS chain.
pub fn dev_dynamo_client(endpoint_url: &str, region: &str) -> aws_sdk_dynamodb::Client {
    let credentials = Credentials::new("test", "test", None, None, "courses-dev");
    let conf = aws_sdk_dynamodb::config::Builder::new()
        .endpoint_url(endpoint_url)
        .region(Region::new(region.to_owned()))
        .credentials_provider(credentials)
        .behavior_version(BehaviorVersion::latest())
        .build();
    aws_sdk_dynamodb::Client::from_conf(conf)
}

/// Creates the apps table if it does not already exist. Idempotent: an
/// existing table is treated as success. Schema: `collection` (S) HASH +
/// `key` (S) RANGE, on-demand billing. Matches the counter handler's writes.
pub async fn ensure_table(client: &aws_sdk_dynamodb::Client, table: &str) -> crate::Result<()> {
    let collection_attr = AttributeDefinition::builder()
        .attribute_name("collection")
        .attribute_type(ScalarAttributeType::S)
        .build()
        .map_err(|e| crate::Error::Dynamo(e.to_string()))?;

    let key_attr = AttributeDefinition::builder()
        .attribute_name("key")
        .attribute_type(ScalarAttributeType::S)
        .build()
        .map_err(|e| crate::Error::Dynamo(e.to_string()))?;

    let hash_key = KeySchemaElement::builder()
        .attribute_name("collection")
        .key_type(KeyType::Hash)
        .build()
        .map_err(|e| crate::Error::Dynamo(e.to_string()))?;

    let range_key = KeySchemaElement::builder()
        .attribute_name("key")
        .key_type(KeyType::Range)
        .build()
        .map_err(|e| crate::Error::Dynamo(e.to_string()))?;

    let result = client
        .create_table()
        .table_name(table)
        .attribute_definitions(collection_attr)
        .attribute_definitions(key_attr)
        .key_schema(hash_key)
        .key_schema(range_key)
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await;

    match result {
        Ok(_) => Ok(()),
        // An already-existing table is fine — local dev reruns are idempotent.
        Err(e) => {
            let svc = e.into_service_error();
            if svc.is_resource_in_use_exception() {
                Ok(())
            } else {
                Err(crate::Error::Dynamo(svc.to_string()))
            }
        }
    }
}
