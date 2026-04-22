use models::{
    portfolio::{Portfolio, Position},
    projects::Project,
};
use poem_openapi::{ApiResponse, Object, Tags, payload::Json};

#[derive(Tags)]
pub enum ApiTags {
    Project,
    Portfolio,
    Position,
}

#[derive(Object)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(ApiResponse)]
pub enum CreateProjectResponse {
    #[oai(status = 201)]
    Created(Json<Project>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetProjectResponse {
    #[oai(status = 200)]
    Ok(Json<Project>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListProjectsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Project>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateProjectResponse {
    #[oai(status = 200)]
    Ok(Json<Project>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeleteProjectResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreatePortfolioResponse {
    #[oai(status = 201)]
    Created(Json<Portfolio>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetPortfolioResponse {
    #[oai(status = 200)]
    Ok(Json<Portfolio>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPortfoliosResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Portfolio>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdatePortfolioResponse {
    #[oai(status = 200)]
    Ok(Json<Portfolio>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeletePortfolioResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreatePositionResponse {
    #[oai(status = 201)]
    Created(Json<Position>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetPositionResponse {
    #[oai(status = 200)]
    Ok(Json<Position>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPositionsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Position>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdatePositionResponse {
    #[oai(status = 200)]
    Ok(Json<Position>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeletePositionResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

pub fn internal_error<E: std::fmt::Display>(err: E) -> Json<ErrorBody> {
    Json(ErrorBody {
        message: err.to_string(),
    })
}
