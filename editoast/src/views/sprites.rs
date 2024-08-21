use axum::body::Body;
use axum::extract::Json;
use axum::extract::Path;
use axum::http::header;
use axum::response::IntoResponse;
use axum::Extension;
use editoast_authz::BuiltinRole;
use editoast_derive::EditoastError;
use thiserror::Error;
use tokio_util::io::ReaderStream;

use crate::client::get_dynamic_assets_path;
use crate::error::Result;
use crate::generated_data::sprite_config::SpriteConfig;
use crate::views::AuthorizationError;
use crate::views::AuthorizerExt;

crate::routes! {
    "/sprites" => {
        "/{signaling_system}/{file_name}" => sprites,
        "/signaling_systems" => signaling_systems,
    },
}

#[derive(Debug, Error, EditoastError)]
#[editoast_error(base_id = "sprites")]
enum SpriteErrors {
    #[error("Unknown signaling system '{signaling_system}'")]
    #[editoast_error(status = 404)]
    UnknownSignalingSystem { signaling_system: String },
    #[error("File '{file}' not found")]
    #[editoast_error(status = 404)]
    FileNotFound { file: String },
}

/// This endpoint returns the list of supported signaling systems
#[utoipa::path(
    get, path = "",
    tag = "sprites",
    responses(
        (status = 200, description = "List of supported signaling systems", body = Vec<String>, example = json!(["BAL", "TVM300"])),
    ),
)]
async fn signaling_systems(Extension(authorizer): AuthorizerExt) -> Result<Json<Vec<String>>> {
    let authorized = authorizer
        .check_roles([BuiltinRole::MapRead].into())
        .await
        .map_err(AuthorizationError::AuthError)?;
    if !authorized {
        return Err(AuthorizationError::Unauthorized.into());
    }

    let sprite_configs = SpriteConfig::load();
    let signaling_systems = sprite_configs.keys().cloned().collect();
    Ok(Json(signaling_systems))
}

/// This endpoint is used by map libre to retrieve the atlas of each signaling system
#[utoipa::path(
    get, path = "",
    tag = "sprites",
    params(
        ("signaling_system" = String, Path, description = "Signaling system name"),
        ("file_name" = String, Path, description = "File name (json, png or svg)"),
    ),
    responses(
        (status = 200, description = "Atlas image of config"),
        (status = 404, description = "Signaling system not found"),
    ),
)]
async fn sprites(
    Extension(authorizer): AuthorizerExt,
    Path((signaling_system, file_name)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let authorized = authorizer
        .check_roles([BuiltinRole::MapRead].into())
        .await
        .map_err(AuthorizationError::AuthError)?;
    if !authorized {
        return Err(AuthorizationError::Unauthorized.into());
    }

    let sprite_configs = SpriteConfig::load();
    if !sprite_configs.contains_key(&signaling_system) {
        return Err(SpriteErrors::UnknownSignalingSystem { signaling_system }.into());
    }
    let path =
        get_dynamic_assets_path().join(format!("signal_sprites/{signaling_system}/{file_name}"));

    if !path.is_file() {
        return Err(SpriteErrors::FileNotFound { file: file_name }.into());
    }
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) => {
            tracing::error!(path = %path.display(), error = %err, "cannot open existing atlas file");
            panic!("Cannot open existing sprite: {}", path.display());
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_type = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .expect("all files have an extension")
    {
        "json" => headers::ContentType::json(),
        "png" => headers::ContentType::png(),
        "svg" => mime::IMAGE_SVG.into(),
        _ => unreachable!(),
    };

    let content_disposition = format!("inline; filename=\"{}\"", file_name);

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        body,
    ))
}

#[cfg(test)]
mod tests {
    use crate::views::test_app::TestAppBuilder;

    use super::*;
    use axum::http::StatusCode;
    use rstest::rstest;

    #[rstest]
    async fn test_signaling_systems() {
        let app = TestAppBuilder::default_app();
        let request = app.get("/sprites/signaling_systems");
        let response: Vec<String> = app.fetch(request).assert_status(StatusCode::OK).json_into();

        assert!(response.contains(&"BAL".to_string()));
        assert!(response.contains(&"BAPR".to_string()));
        assert!(response.contains(&"TVM300".to_string()));
        assert!(response.contains(&"TVM430".to_string()));
    }

    #[rstest]
    async fn test_sprites() {
        let app = TestAppBuilder::default_app();
        let request = app.get("/sprites/TVM300/REP%20TGV.svg");
        let response = app.fetch(request).assert_status(StatusCode::OK);
        assert_eq!("image/svg+xml", response.content_type());
        let response = response.bytes();
        let expected =
            std::fs::read(get_dynamic_assets_path().join("signal_sprites/TVM300/REP TGV.svg"))
                .unwrap();
        assert_eq!(response, expected);
    }

    #[rstest]
    async fn test_sprites_not_found() {
        let app = TestAppBuilder::default_app();
        let request = app.get("/sprites/TVM300/NOT_A_THING.svg");
        app.fetch(request).assert_status(StatusCode::NOT_FOUND);
    }
}
