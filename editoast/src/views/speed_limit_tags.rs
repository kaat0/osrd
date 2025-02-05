use axum::extract::State;
use axum::Extension;
use axum::Json;
use editoast_authz::BuiltinRole;

use crate::error::Result;
use crate::generated_data::speed_limit_tags_config::SpeedLimitTagIds;
use crate::views::AuthenticationExt;
use crate::views::AuthorizationError;
use crate::AppState;

crate::routes! {
    "/speed_limit_tags" => speed_limit_tags,
}

#[utoipa::path(
    get, path = "",
    tag = "speed_limit_tags",
    responses(
        (status = 200, description = "List of configured speed-limit tags", body = Vec<String>, example = json!(["V200", "MA80"])),
    ),
)]
async fn speed_limit_tags(
    State(AppState {
        speed_limit_tag_ids,
        ..
    }): State<AppState>,
    Extension(auth): AuthenticationExt,
) -> Result<Json<SpeedLimitTagIds>> {
    let authorized = auth
        .check_roles([BuiltinRole::RollingStockCollectionRead].into())
        .await
        .map_err(AuthorizationError::AuthError)?;
    if !authorized {
        return Err(AuthorizationError::Forbidden.into());
    }

    Ok(Json(speed_limit_tag_ids.as_ref().clone()))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use rstest::rstest;

    use crate::views::test_app::TestAppBuilder;

    #[rstest]
    async fn get_speed_limit_list() {
        let app = TestAppBuilder::default_app();

        let request = app.get("/speed_limit_tags");

        let response: Vec<String> = app.fetch(request).assert_status(StatusCode::OK).json_into();

        assert!(response.len() >= 2);
        assert!(response.contains(&"MA80".to_string()));
        assert!(response.contains(&"V200".to_string()));
    }
}
