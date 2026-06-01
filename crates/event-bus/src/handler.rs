use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EventBusError;
use crate::event::Event;

#[async_trait]
pub trait EventHandler<E: Event>: Send + Sync {
    async fn handle(&self, event: &E) -> Result<(), EventBusError>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerResult {
    pub handler_name: String,
    pub success: bool,
    pub error: Option<String>,
}

impl HandlerResult {
    pub fn ok(name: &str) -> Self {
        Self {
            handler_name: name.to_string(),
            success: true,
            error: None,
        }
    }

    pub fn err(name: &str, error: &str) -> Self {
        Self {
            handler_name: name.to_string(),
            success: false,
            error: Some(error.to_string()),
        }
    }
}

pub trait HandlerEraser: Send + Sync {
    fn handle_erased(&self, event_json: &str, event_type: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EventBusError>> + Send + '_>>;
    fn name(&self) -> &str;
}

pub struct ErasedHandler<E: Event> {
    inner: Box<dyn EventHandler<E>>,
    _marker: std::marker::PhantomData<E>,
}

impl<E: Event> ErasedHandler<E> {
    pub fn new(handler: Box<dyn EventHandler<E>>) -> Self {
        Self {
            inner: handler,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E: Event> HandlerEraser for ErasedHandler<E> {
    fn handle_erased(&self, event_json: &str, _event_type: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EventBusError>> + Send + '_>> {
        let event: E = match E::from_json(event_json) {
            Ok(e) => e,
            Err(err) => {
                return Box::pin(async move { Err(err) });
            }
        };
        let inner = &self.inner;
        Box::pin(async move { inner.handle(&event).await })
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}
