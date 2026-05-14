//! OTLP gRPC receiver (`:4317` style) for logs and traces.

use std::net::SocketAddr;
use std::sync::Arc;

use mara_core::traits::EventSender;
use opentelemetry_proto::tonic::collector::logs::v1::logs_service_server::{LogsService, LogsServiceServer};
use opentelemetry_proto::tonic::collector::logs::v1::{ExportLogsServiceRequest, ExportLogsServiceResponse};
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_server::{TraceService, TraceServiceServer};
use opentelemetry_proto::tonic::collector::trace::v1::{ExportTraceServiceRequest, ExportTraceServiceResponse};
use tokio::sync::Notify;
use tonic::{Request, Response, Status};

use crate::http::{dispatch_export_logs, dispatch_export_traces};

#[derive(Clone)]
struct OtlpGrpcLogs {
    out: EventSender,
    adapter_name: String,
}

#[tonic::async_trait]
impl LogsService for OtlpGrpcLogs {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let _ = dispatch_export_logs(request.into_inner(), &self.out, &self.adapter_name).await;
        Ok(Response::new(ExportLogsServiceResponse::default()))
    }
}

#[derive(Clone)]
struct OtlpGrpcTraces {
    out: EventSender,
    adapter_name: String,
}

#[tonic::async_trait]
impl TraceService for OtlpGrpcTraces {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let _ = dispatch_export_traces(request.into_inner(), &self.out, &self.adapter_name).await;
        Ok(Response::new(ExportTraceServiceResponse::default()))
    }
}

/// Run until `stop` is notified (shared with the HTTP server adapter).
pub async fn serve(
    addr: SocketAddr,
    out: EventSender,
    adapter_name: String,
    stop: Arc<Notify>,
) -> Result<(), tonic::transport::Error> {
    let logs = OtlpGrpcLogs { out: out.clone(), adapter_name: adapter_name.clone() };
    let traces = OtlpGrpcTraces { out, adapter_name };

    tonic::transport::Server::builder()
        .add_service(LogsServiceServer::new(logs))
        .add_service(TraceServiceServer::new(traces))
        .serve_with_shutdown(addr, async move {
            stop.notified().await;
        })
        .await
}
