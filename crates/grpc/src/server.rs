use bytes::Bytes;
use common::error::FerroError;
use common::metadata::FileMetadata;
use common::storage::StorageEngine;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::proto::ferro_storage_server::{FerroStorage, FerroStorageServer};
use crate::proto::{
    DeleteFileRequest, DeleteFileResponse, GetFileRequest, GetFileResponse, HeadFileRequest,
    HeadFileResponse, ListFilesRequest, ListFilesResponse, PutFileRequest, PutFileResponse,
};

#[derive(Clone)]
pub struct FerroGrpcService {
    engine: Arc<dyn StorageEngine>,
}

impl FerroGrpcService {
    pub fn new(engine: Arc<dyn StorageEngine>) -> Self {
        Self { engine }
    }

    pub fn into_server(self) -> FerroStorageServer<Self> {
        FerroStorageServer::new(self)
    }
}

fn metadata_to_proto(m: &FileMetadata) -> crate::proto::FileMetadata {
    crate::proto::FileMetadata {
        path: m.path.clone(),
        content_hash: m.content_hash.as_str().to_string(),
        size: m.size,
        mime_type: m.mime_type.clone(),
        is_collection: m.is_collection,
        created_at: m.created_at.timestamp(),
        modified_at: m.modified_at.timestamp(),
        owner: m.owner.clone(),
        etag: m.etag.clone(),
    }
}

fn ferro_error_to_status(err: FerroError) -> Status {
    match err {
        FerroError::NotFound(msg) => Status::not_found(msg),
        FerroError::AlreadyExists(msg) => Status::already_exists(msg),
        FerroError::PermissionDenied(msg) => Status::permission_denied(msg),
        FerroError::InvalidArgument(msg) => Status::invalid_argument(msg),
        FerroError::Internal(msg) => Status::internal(msg),
        FerroError::Unauthorized => Status::unauthenticated("authentication required"),
        _ => Status::internal(err.to_string()),
    }
}

#[tonic::async_trait]
impl FerroStorage for FerroGrpcService {
    async fn put_file(
        &self,
        request: Request<PutFileRequest>,
    ) -> Result<Response<PutFileResponse>, Status> {
        let req = request.into_inner();
        let content = req.content.map(|c| Bytes::from(c.data)).unwrap_or_default();
        let meta = self
            .engine
            .put(&req.path, content, &req.owner)
            .await
            .map_err(ferro_error_to_status)?;
        Ok(Response::new(PutFileResponse {
            metadata: Some(metadata_to_proto(&meta)),
        }))
    }

    async fn get_file(
        &self,
        request: Request<GetFileRequest>,
    ) -> Result<Response<GetFileResponse>, Status> {
        let req = request.into_inner();
        let data = self
            .engine
            .get(&req.path)
            .await
            .map_err(ferro_error_to_status)?;
        let meta = self
            .engine
            .head(&req.path)
            .await
            .map_err(ferro_error_to_status)?;
        Ok(Response::new(GetFileResponse {
            content: Some(crate::proto::FileContent {
                data: data.to_vec(),
            }),
            metadata: Some(metadata_to_proto(&meta)),
        }))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        self.engine
            .delete(&req.path)
            .await
            .map_err(ferro_error_to_status)?;
        Ok(Response::new(DeleteFileResponse {}))
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        let entries = self
            .engine
            .list(&req.path)
            .await
            .map_err(ferro_error_to_status)?;
        Ok(Response::new(ListFilesResponse {
            entries: entries.iter().map(metadata_to_proto).collect(),
        }))
    }

    async fn head_file(
        &self,
        request: Request<HeadFileRequest>,
    ) -> Result<Response<HeadFileResponse>, Status> {
        let req = request.into_inner();
        let meta = self
            .engine
            .head(&req.path)
            .await
            .map_err(ferro_error_to_status)?;
        Ok(Response::new(HeadFileResponse {
            metadata: Some(metadata_to_proto(&meta)),
        }))
    }
}
