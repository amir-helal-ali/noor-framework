// ============================================================
// Noor Framework - Main Library Entry Point
// نقطة الدخول الرئيسية لإطار عمل نور
// ============================================================
// Noor is a high-performance, secure, fullstack MVC framework
// that runs efficiently even on weak servers (512MB RAM).
//
// نور إطار عمل عالي الأداء وآمن يعمل بكفاءة حتى على
// السيرفرات الضعيفة (512 ميجا رام).
// ============================================================

// NOTE: The blanket `#![allow(warnings)]` / `#![allow(unused_*)]` that used
// to live here masked real bugs (including a deadlock and 23 failing tests).
// They have been removed so that `cargo check` surfaces issues honestly.
//
// We keep a *targeted* `#![allow(dead_code)]` because this is a framework
// library: many public types/functions are part of the API surface but are
// not exercised by internal callers, and that is intentional. All other
// warning categories (unused_imports, unused_variables, deprecated, etc.)
// remain enabled so genuine issues stay visible.
#![allow(dead_code)]

pub mod core;
pub mod demo;

// Re-export the most commonly used items for convenience
pub use core::{
    application::Application,
    config::Config,
    http::{Request, Response, Method, StatusCode},
    router::{Router, Route, Group},
    middleware::{Middleware, MiddlewareStack},
    orm::{Model, QueryBuilder, Database, Migration},
    auth::{Jwt, Session, Rbac, Guard},
    security::{Csrf, Xss, RateLimit, Encryption, Validator},
    cache::{Cache, FileCache, MemoryCache, CacheManager},
    view::{Template, ViewEngine},
    cli::Cli,
    server::Server,
    websocket::{WebSocketServer, WsMessage, Connection as WsConnection},
    events::{EventEmitter, Event},
    queue::{Queue, Job as QueueJob, Priority as JobPriority},
    mail::{Mailer, Email, MailConfig, MailDriver},
    upload::{FileUploader, UploadConfig, UploadedFile},
    scheduler::{Scheduler, Schedule},
    seeder::{Seeder, Factory as SeederFactory},
    testing::{TestClient, TestResponse},
    plugins::{PluginManager, Plugin, PluginInfo, DebugPlugin, StatsPlugin},
    metrics::{MetricsRegistry, AppMetrics, Counter, Gauge, Histogram, Timer},
    storage::{Storage, LocalStorage, S3Storage, StorageManager, Visibility as StorageVisibility},
    pagination::{Pagination, PaginatedResult, PaginationParams, Sort, SortDirection},
    health::{HealthChecker, HealthReport, Status as HealthStatus, CheckResult},
    graphql::{GraphQLResolver, GraphQLResponse, SchemaBuilder, FieldBuilder, GraphQLType},
    container::{Container, ServiceLifetime},
    i18n::{Translator, TextDirection, translator as i18n_translator, t, tl},
    openapi::{OpenApiBuilder, OpenApiSpec, OperationBuilder, SchemaBuilder as OpenApiSchemaBuilder},
    admin::{AdminGenerator, ScaffoldBuilder, ScaffoldConfig, FieldBuilder as AdminFieldBuilder, FieldType},
    command::{CommandBus, Command, CommandHandler, SimpleCommand},
    webhook::{WebhookManager, Webhook, WebhookPayload},
    validation::{Validator as RequestValidator, RuleBuilder as ValidationRuleBuilder, ValidationResult},
    versioning::{VersionManager, VersioningStrategy},
    backup::{BackupManager, BackupConfig, BackupType, BackupInfo},
    profiler::{Profiler, Profile, ProfileSection, ProfileSummary},
    notification::{NotificationManager, Notification, Channel as NotificationChannel, NotificationPreferences, ChannelHandler},
    search::{SearchEngine, SearchIndex, SearchDocument, SearchQuery, SearchResult},
    image::{ImageProcessor, ImageFormat, Dimensions, ResizeOptions, WatermarkOptions},
    features::{FeatureFlagManager, FeatureFlag, FlagStrategy},
    tenancy::{TenantManager, Tenant, TenantPlan, TenantStatus, ResolutionStrategy as TenantResolutionStrategy, TenantScope},
    oauth::{OAuthManager, OAuthProvider, OAuthUser},
    assets::{AssetPipeline, AssetConfig, AssetType, AssetEntry},
    audit::{AuditLogger, AuditEntry, AuditSeverity},
    provider::{ProviderManager, ServiceProvider},
    repository::{Repository, InMemoryRepository, RepositoryFactory},
    resource::{ApiResource, ResourceCollection, JsonResponse, ResourceMeta, ResourceLinks},
    form_request::{FormRequest, FormRequestResult},
    model_binding::{ModelBinder, ModelResolver, ImplicitBinding},
    state_machine::{StateMachine, StateMachineInstance, Transition},
    observer::{Observer, ObserverManager, ObserverRegistry},
    circuit_breaker::{CircuitBreaker, CircuitState, CircuitConfig, CircuitBreakerRegistry, CircuitBreakerError},
    env::{EnvLoader, env, env_value, env_or, env_int, env_bool},
    dto::{Dto, DtoError, DtoCollection, CreateUserDto, UpdateUserDto, LoginDto, ResponseDto, PaginationDto},
    cookies::{Cookie, CookieJar, SameSite, SignedCookieManager, EncryptedCookieManager},
    session_drivers::{SessionDriver, SessionData, SessionManager as SessionDriverManager, FileSessionDriver, MemorySessionDriver},
    http_client::{HttpClient, HttpRequest, HttpResponse, HttpMethod, RequestBuilder},
    error_pages::render_error,
    test_doubles::{Mock, Stub, FakeDatabase, Spy, Fixture},
    config_manager::{ConfigManager, ConfigSource, config, cfg, cfg_string, cfg_or},
    logging_drivers::{Logger, LogLevel, LogRecord, LogDriver, ConsoleDriver, FileDriver, MemoryDriver},
    console::{Console, CommandBuilder, Command as ConsoleCommand, CommandInput, Argument, Option_ as ConsoleOption},
    pool::{ConnectionPool, PoolConfig, PoolStats, Connection},
    advanced_query::AdvancedQueryBuilder,
    key_rotation::{KeyRotationManager, EncryptionKey, KeyStatus, EncryptedData, KeyRotationStats},
    rate_limiter_advanced::{AdvancedRateLimiter, RateLimitConfig, RateLimitStrategy, RateLimitResult, RateLimiterStats},
    file_watcher::{FileWatcher, FileChangeEvent, FileEventType},
    transactions::{TransactionManager, IsolationLevel, TransactionStats, TransactionBuilder},
    soft_deletes::{SoftDeletes, SoftDeletesMixin, SoftDeleteScope},
    scopes::{ScopeRegistry, register_common_scopes},
    accessors::{AttributeManager, common as accessor_common},
    query_cache::{QueryCache, QueryCacheStats},
    pipeline::{Pipeline, PipelineBuilder, PipelineAction},
    sse::{SseServer, SseEvent, SseClient, SseChannel},
    streaming::{FileStreamer, ByteRange, StreamMeta, StreamResponse},
    wasm::{WasmManager, WasmModule, WasmPlugin},
    tracing::{Tracer, Span, SpanKind, SpanStatus, TraceContext, TracerStats},
    csp::{ContentSecurityPolicy, CspDirective, CspSource},
    factory_advanced::{FactoryDefinition, FactoryRegistry, faker},
    repl::{Repl, ReplContext, ReplCommand},
    brotli::{compress_response, CompressionType, CompressionConfig, compression_ratio},
    cqrs::{CqrsSystem, CommandBus as CqrsCommandBus, QueryBus, Command as CqrsCommand, CommandHandler as CqrsCommandHandler, Query, QueryHandler},
    event_sourcing::{EventStore, InMemoryEventStore, AggregateRoot, EventSourcingRepository, EventFactory, DomainEvent, Snapshot, SnapshotStore},
    distributed_lock::{LockManager, MemoryLockManager, FileLockManager, LockResult, LockGuard, DistributedLock},
    service_discovery::{ServiceRegistry, ServiceInstance, LoadBalancer, LoadBalancerStrategy, ServiceRegistryStats},
    api_gateway::{ApiGateway, GatewayRoute, GatewayMiddleware},
    idempotency::{IdempotencyManager, IdempotencyCheck, IdempotencyStatus, IdempotencyRecord},
    outbox::{OutboxStore, InMemoryOutboxStore, OutboxMessage, OutboxProcessor, OutboxStatus, OutboxStats},
    saga::{Saga, SagaStep, SagaResult, SagaState, SagaStatus, SagaOrchestrator},
};

// Common result and error types
pub use core::error::{NoorResult, NoorError};

/// The version of the Noor Framework
/// إصدار إطار عمل نور
pub const VERSION: &str = "1.0.0";

/// Returns the framework banner
/// يرجع شعار الإطار
pub fn banner() -> String {
    format!(
        r#"
  ███╗   ██╗ ██████╗ ██╗  ██╗██████╗ 
  ████╗  ██║██╔═══██╗██║  ██║██╔══██╗
  ██╔██╗ ██║██║   ██║██████╔╝██████╔╝
  ██║╚██╗██║██║   ██║██╔═══╝ ██╔═══╝ 
  ██║ ╚████║╚██████╔╝██║     ██║     
  ╚═╝  ╚═══╝ ╚═════╝ ╚═╝     ╚═╝     
  
  Noor Framework v{} - Light. Fast. Secure.
  إطار عمل نور - خفيف، سريع، آمن
"#,
        VERSION
    )
}
