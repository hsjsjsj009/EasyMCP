use std::marker::PhantomData;
use std::sync::Arc;
use futures_core::future::BoxFuture;
use rmcp::handler::server::tool::{CallToolHandler, FromToolCallContextPart, IntoCallToolResult, ToolCallContext};
use rmcp::model::{CallToolResult, ErrorData};

#[derive(Clone)]
pub struct DynamicMCPClosure<S,R> {
    closure: Arc<dyn Fn(S) -> BoxFuture<'static, R> + Send + Sync + 'static>
}

impl<S,R> DynamicMCPClosure<S,R> {
    pub fn new(closure: impl Fn(S) -> BoxFuture<'static, R> + Send + Sync + 'static) -> Self {
        Self {
            closure: Arc::new(closure)
        }
    }
}

impl<T0, S, R> CallToolHandler<S,PhantomData<dyn Fn(T0) -> R>> for DynamicMCPClosure<T0,R> where
    T0: FromToolCallContextPart<S>,
    S: Send + Sync + 'static,
    R: IntoCallToolResult + Send + 'static,
{
    fn call(self, mut context: ToolCallContext<'_, S>) -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
        let result =  T0::from_tool_call_context_part(&mut context);
        let request = match result {
            Ok(request) => request,
            Err(err) => return Box::pin(async move { Err(err) })
        };
        let result = (self.closure)(request);
        Box::pin(async move { result.await.into_call_tool_result() })
    }
}