use tracing::Subscriber;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

/// 将多个层次组合成 `tracing` 的订阅器
///
/// # 注意事项
///
/// 将 `impl Subscriber` 作为返回值类型，以免写出繁琐的真实类型
/// 我们需要显式将返回类型标记为 `Send` + `Sync`， 以便后面可以将其传递给 `init_subscriber`
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    // 高阶特质约束
    // 意思是 `Sink` 会实现 `MakeWriter` 特型， 无论生命周期 `'a` 是什么
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    //如果没有设置环境变量RUST_LOG，则使用info级别
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// 将一个订阅器设置为全局默认值，用于处理所有跨度数据
///
/// 这个函数只可被调用一次
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
