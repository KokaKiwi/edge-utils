-- ClickHouse Views to map dev.otel_traces to Jaeger-compatible schema
-- This bridges the OTel Collector schema (Map-based attributes) to Jaeger v2 schema (Nested typed arrays)
-- Note: This is a simplified mapping for dev purposes

CREATE DATABASE IF NOT EXISTS jaeger;

-- Main spans view - maps otel_traces to Jaeger's expected spans table format
CREATE OR REPLACE VIEW jaeger.spans AS
SELECT
    SpanId AS id,
    TraceId AS trace_id,
    TraceState AS trace_state,
    ParentSpanId AS parent_span_id,
    SpanName AS name,
    SpanKind AS kind,
    Timestamp AS start_time,
    StatusCode AS status_code,
    StatusMessage AS status_message,
    Duration AS duration,

    -- Span attributes as string key-value pairs
    mapKeys(SpanAttributes) AS `str_attributes.key`,
    mapValues(SpanAttributes) AS `str_attributes.value`,

    -- Empty arrays for typed attributes (simplified for dev)
    CAST([] AS Array(String)) AS `bool_attributes.key`,
    CAST([] AS Array(UInt8)) AS `bool_attributes.value`,
    CAST([] AS Array(String)) AS `double_attributes.key`,
    CAST([] AS Array(Float64)) AS `double_attributes.value`,
    CAST([] AS Array(String)) AS `int_attributes.key`,
    CAST([] AS Array(Int64)) AS `int_attributes.value`,
    CAST([] AS Array(String)) AS `complex_attributes.key`,
    CAST([] AS Array(String)) AS `complex_attributes.value`,

    -- Events - map from OTel arrays to Jaeger Nested format
    `Events.Name` AS `events.name`,
    `Events.Timestamp` AS `events.timestamp`,
    -- Transform Array(Map) attributes to Nested arrays for Jaeger
    arrayMap(attrs -> arrayMap(k -> k, mapKeys(attrs)), `Events.Attributes`) AS `events.str_attributes.key`,
    arrayMap(attrs -> arrayMap(v -> v, mapValues(attrs)), `Events.Attributes`) AS `events.str_attributes.value`,
    -- Create empty arrays for typed attributes, matching the number of events
    arrayMap(x -> CAST([] AS Array(String)), `Events.Name`) AS `events.bool_attributes.key`,
    arrayMap(x -> CAST([] AS Array(UInt8)), `Events.Name`) AS `events.bool_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Events.Name`) AS `events.double_attributes.key`,
    arrayMap(x -> CAST([] AS Array(Float64)), `Events.Name`) AS `events.double_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Events.Name`) AS `events.int_attributes.key`,
    arrayMap(x -> CAST([] AS Array(Int64)), `Events.Name`) AS `events.int_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Events.Name`) AS `events.complex_attributes.key`,
    arrayMap(x -> CAST([] AS Array(String)), `Events.Name`) AS `events.complex_attributes.value`,

    -- Links - map from OTel arrays to Jaeger Nested format
    `Links.TraceId` AS `links.trace_id`,
    `Links.SpanId` AS `links.span_id`,
    `Links.TraceState` AS `links.trace_state`,
    -- Transform Array(Map) attributes to Nested arrays for Jaeger
    arrayMap(attrs -> arrayMap(k -> k, mapKeys(attrs)), `Links.Attributes`) AS `links.str_attributes.key`,
    arrayMap(attrs -> arrayMap(v -> v, mapValues(attrs)), `Links.Attributes`) AS `links.str_attributes.value`,
    -- Create empty arrays for typed attributes, matching the number of links
    arrayMap(x -> CAST([] AS Array(String)), `Links.TraceId`) AS `links.bool_attributes.key`,
    arrayMap(x -> CAST([] AS Array(UInt8)), `Links.TraceId`) AS `links.bool_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Links.TraceId`) AS `links.double_attributes.key`,
    arrayMap(x -> CAST([] AS Array(Float64)), `Links.TraceId`) AS `links.double_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Links.TraceId`) AS `links.int_attributes.key`,
    arrayMap(x -> CAST([] AS Array(Int64)), `Links.TraceId`) AS `links.int_attributes.value`,
    arrayMap(x -> CAST([] AS Array(String)), `Links.TraceId`) AS `links.complex_attributes.key`,
    arrayMap(x -> CAST([] AS Array(String)), `Links.TraceId`) AS `links.complex_attributes.value`,

    -- Service and resource attributes
    ServiceName AS service_name,
    mapKeys(ResourceAttributes) AS `resource_str_attributes.key`,
    mapValues(ResourceAttributes) AS `resource_str_attributes.value`,
    CAST([] AS Array(String)) AS `resource_bool_attributes.key`,
    CAST([] AS Array(UInt8)) AS `resource_bool_attributes.value`,
    CAST([] AS Array(String)) AS `resource_double_attributes.key`,
    CAST([] AS Array(Float64)) AS `resource_double_attributes.value`,
    CAST([] AS Array(String)) AS `resource_int_attributes.key`,
    CAST([] AS Array(Int64)) AS `resource_int_attributes.value`,
    CAST([] AS Array(String)) AS `resource_complex_attributes.key`,
    CAST([] AS Array(String)) AS `resource_complex_attributes.value`,

    -- Scope/instrumentation library
    ScopeName AS scope_name,
    ScopeVersion AS scope_version,
    CAST([] AS Array(String)) AS `scope_bool_attributes.key`,
    CAST([] AS Array(UInt8)) AS `scope_bool_attributes.value`,
    CAST([] AS Array(String)) AS `scope_double_attributes.key`,
    CAST([] AS Array(Float64)) AS `scope_double_attributes.value`,
    CAST([] AS Array(String)) AS `scope_int_attributes.key`,
    CAST([] AS Array(Int64)) AS `scope_int_attributes.value`,
    CAST([] AS Array(String)) AS `scope_str_attributes.key`,
    CAST([] AS Array(String)) AS `scope_str_attributes.value`,
    CAST([] AS Array(String)) AS `scope_complex_attributes.key`,
    CAST([] AS Array(String)) AS `scope_complex_attributes.value`
FROM default.otel_traces;

-- Services view - extract unique service names
CREATE OR REPLACE VIEW jaeger.services AS
SELECT DISTINCT
    ServiceName AS name
FROM default.otel_traces
WHERE ServiceName != '';

-- Operations view - extract unique operation (span) names per service
CREATE OR REPLACE VIEW jaeger.operations AS
SELECT DISTINCT
    ServiceName AS service_name,
    SpanName AS name,
    SpanKind AS span_kind
FROM default.otel_traces
WHERE ServiceName != '' AND SpanName != '';

-- Trace ID timestamps view - for efficient trace lookup
CREATE OR REPLACE VIEW jaeger.trace_id_timestamps AS
SELECT
    TraceId AS trace_id,
    min(Timestamp) AS start,
    max(Timestamp) AS end
FROM default.otel_traces
GROUP BY TraceId;

-- Attribute metadata view (optional - for Jaeger's attribute search)
CREATE OR REPLACE VIEW jaeger.attribute_metadata AS
SELECT DISTINCT
    'span' AS scope,
    key AS attribute_key,
    'string' AS attribute_type
FROM default.otel_traces
ARRAY JOIN mapKeys(SpanAttributes) AS key
UNION ALL
SELECT DISTINCT
    'resource' AS scope,
    key AS attribute_key,
    'string' AS attribute_type
FROM default.otel_traces
ARRAY JOIN mapKeys(ResourceAttributes) AS key;
