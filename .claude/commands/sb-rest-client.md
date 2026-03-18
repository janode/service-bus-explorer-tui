---
description: 'Extend the Azure Service Bus REST API client. Use when: add API call, new REST endpoint, add management API, add data plane API, ATOM XML parsing, new entity type, extend client, HTTP request to Service Bus, XML feed parsing.'
---

# Azure Service Bus REST Client Extension

Extends the custom REST client for Azure Service Bus. This project uses **no Azure SDK** — all API calls use `reqwest` against the REST API directly, with ATOM XML for management operations and JSON for data plane operations.

## Architecture

| Module | Plane | Format | Purpose |
|--------|-------|--------|---------|
| `src/client/management.rs` | Management | ATOM XML | Entity CRUD (queues, topics, subscriptions) |
| `src/client/data_plane.rs` | Data plane | JSON/HTTP | Messages (send, peek, receive, delete, purge) |
| `src/client/auth.rs` | Both | — | SAS token generation, Azure AD tokens |
| `src/client/models.rs` | Both | — | Data structures for entities and messages |
| `src/client/error.rs` | Both | — | `ServiceBusError` enum (thiserror) |

## Management Plane (ATOM XML)

### Adding a new management operation

**Step 1:** Define the model in `src/client/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MyEntityDescription {
    pub name: String,
    pub some_property: Option<String>,
    pub some_flag: Option<bool>,
    pub some_count: Option<i64>,
}
```

**Convention:** All properties except `name` are `Option<T>` because Azure may omit them from responses.

**Step 2:** Add XML building function in `src/client/management.rs`:

```rust
fn my_entity_description_xml(desc: &MyEntityDescription) -> String {
    let mut xml = String::from(
        r#"<MyEntityDescription xmlns="http://schemas.microsoft.com/netservices/2010/10/servicebus/connect" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">"#,
    );
    if let Some(ref v) = desc.some_property {
        xml.push_str(&format!("<SomeProperty>{}</SomeProperty>", v));
    }
    if let Some(v) = desc.some_flag {
        xml.push_str(&format!("<SomeFlag>{}</SomeFlag>", v));
    }
    xml.push_str("</MyEntityDescription>");
    xml
}
```

**Important:** Element names are PascalCase and must match the Azure API schema exactly. Wrap with `wrap_atom_entry()` before sending.

**Step 3:** Add XML parsing function:

```rust
fn parse_my_entity_from_entry(entry_xml: &str) -> MyEntityDescription {
    let name = extract_title(entry_xml);
    MyEntityDescription {
        name,
        some_property: extract_element_value(entry_xml, "SomeProperty"),
        some_flag: parse_optional_bool(entry_xml, "SomeFlag"),
        some_count: parse_optional_i64(entry_xml, "SomeCount"),
    }
}
```

**Step 4:** Add CRUD methods on `ManagementClient`:

```rust
impl ManagementClient {
    pub async fn list_my_entities(&self) -> Result<Vec<MyEntityDescription>> {
        let xml = self.get_atom("$Resources/MyEntities").await?;
        Ok(extract_entries(&xml)
            .into_iter()
            .map(|e| parse_my_entity_from_entry(&e))
            .collect())
    }

    pub async fn get_my_entity(&self, name: &str) -> Result<MyEntityDescription> {
        let xml = self.get_atom(name).await?;
        Ok(parse_my_entity_from_entry(&xml))
    }

    pub async fn create_my_entity(&self, desc: &MyEntityDescription) -> Result<MyEntityDescription> {
        let inner = my_entity_description_xml(desc);
        let body = wrap_atom_entry(&inner);
        let xml = self.put_atom(&desc.name, &body).await?;
        Ok(parse_my_entity_from_entry(&xml))
    }

    pub async fn delete_my_entity(&self, name: &str) -> Result<()> {
        self.delete_entity(name).await
    }
}
```

### XML Parsing Helpers

The project uses raw string extraction instead of serde XML because Azure returns inconsistent ATOM feed schemas.

| Helper | Purpose | Handles Attributes? |
|--------|---------|---------------------|
| `extract_entries(xml)` | Split feed into `<entry>` strings | N/A |
| `extract_title(xml)` | Get `<title type="text">` content | Yes |
| `extract_element(xml, tag)` | Get element content (tag may have attributes) | Yes |
| `extract_element_value(xml, tag)` | Get `<tag>content</tag>` (exact open tag) | No |
| `extract_value_any_ns(xml, local)` | Get element ignoring namespace prefix | Yes (any prefix) |
| `parse_optional_i64/i32/bool` | Parse typed values | Via `extract_element_value` |
| `parse_count_details(xml)` | Extract `CountDetails` (active, DLQ, scheduled, ...) | Via `extract_value_any_ns` |

**Namespace prefix pitfall:** Azure's WCF serializer uses auto-generated prefixes (`d2p1:`, `d3p1:`) that vary by nesting depth. Use `extract_value_any_ns()` for elements inside `CountDetails` or other nested containers.

## Data Plane (JSON/HTTP)

### Adding a new data plane operation

In `src/client/data_plane.rs`:

```rust
impl DataPlaneClient {
    pub async fn my_operation(&self, entity_path: &str) -> Result<MyResult> {
        let path = Self::normalize_path(entity_path);
        let url = format!("{}/{}/messages?api-version=2017-04", self.config.endpoint, path);
        let token = self.config.namespace_token().await?;

        let resp = self.http
            .post(&url)
            .header("Authorization", token)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await?;
            return Err(ServiceBusError::Api { status, body });
        }
        // Parse response...
        Ok(result)
    }
}
```

**Critical:** Always call `Self::normalize_path()` on entity paths. This converts management-style PascalCase (`/Subscriptions/`) to data-plane lowercase (`/subscriptions/`).

### Path conventions

| Entity | Management path | Data plane path |
|--------|----------------|-----------------|
| Queue | `my-queue` | `my-queue` |
| Topic | `my-topic` | `my-topic` |
| Subscription | `my-topic/Subscriptions/my-sub` | `my-topic/subscriptions/my-sub` |
| Queue DLQ | N/A | `my-queue/$deadletterqueue` |
| Subscription DLQ | N/A | `my-topic/subscriptions/my-sub/$deadletterqueue` |

### Data plane message headers

| Header | Purpose |
|--------|---------|
| `BrokerProperties` | JSON object with `MessageId`, `CorrelationId`, `SessionId`, `Label`, `TimeToLive`, etc. |
| `Content-Type` | Message content type |
| Custom properties | Any additional header becomes a custom property |

## Error Handling

All client methods return `Result<T>` using the crate's error type:

```rust
// src/client/error.rs
pub enum ServiceBusError {
    Auth(String),
    Api { status: u16, body: String },
    NotFound(String),
    Xml(String),
    Http(reqwest::Error),
}
```

- Return `ServiceBusError::Api` for non-404 HTTP errors (include status + body for debugging)
- Return `ServiceBusError::NotFound` for 404s
- Return `ServiceBusError::Xml` for parse failures (include what was expected + what was found)
- `reqwest::Error` auto-converts via `From` impl

## Checklist

- [ ] Model struct added to `models.rs` with `Option` fields
- [ ] XML builder function accepts `&ModelDescription` and outputs XML string
- [ ] XML parser function extracts fields using the parsing helpers
- [ ] `ManagementClient` or `DataPlaneClient` method added
- [ ] Path casing correct (PascalCase for management, lowercase for data plane)
- [ ] `normalize_path()` called in data plane methods
- [ ] `namespace_token().await?` used for auth headers
- [ ] Errors return appropriate `ServiceBusError` variant
- [ ] 404 responses mapped to `ServiceBusError::NotFound`
