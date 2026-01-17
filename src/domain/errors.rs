use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("No encontrado: {0}")]
    NotFound(String),
    #[error("Entrada inválida: {0}")]
    InvalidInput(String),
    #[error("Error de operación: {0}")]
    OperationFailed(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
