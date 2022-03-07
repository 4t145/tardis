use serde::{Deserialize, Serialize};

use tardis::basic::error::TardisError;
use tardis::basic::field::TrimString;
use tardis::db::sea_orm::*;
use tardis::db::sea_query::Query as DbQuery;
use tardis::web::context_extractor::TardisContextExtractor;
use tardis::web::poem_openapi::param::Query;
use tardis::web::poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use tardis::web::web_resp::{TardisApiResult, Void};
use tardis::web::web_resp::{TardisPage, TardisResp};
use tardis::TardisFuns;

use crate::domain::todos;

#[derive(Object, FromQueryResult, Serialize, Deserialize, Debug)]
struct TodoDetailResp {
    id: i32,
    code: String,
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug)]
struct TodoAddReq {
    #[oai(validator(min_length = "2", max_length = "255"))]
    code: TrimString,
    #[oai(validator(min_length = "2", max_length = "255"))]
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug)]
struct TodoModifyReq {
    #[oai(validator(min_length = "2", max_length = "255"))]
    description: Option<String>,
    done: Option<bool>,
}

pub struct TodoApi;

#[OpenApi(prefix_path = "/todo")]
impl TodoApi {
    // curl -X POST "http://127.0.0.1:8089/todo" \
    //  -H "Accept: application/json" \
    //  -H "Content-Type: application/json" \
    //  -H "Tardis-Context: eyJhcHBfaWQiOiAiIiwidGVuYW50X2lkIjogIiIsImFrIjogIiIsImFjY291bnRfaWQiOiAiIiwidG9rZW4iOiAiIiwidG9rZW5fa2luZCI6ICIiLCJyb2xlcyI6IFtdLCJncm91cHMiOiBbXX0=" \
    //  -d '{"code":"  测试2  ","description":"AA","done":false}'
    #[oai(path = "/", method = "post")]
    async fn add(&self, todo_add_req: Json<TodoAddReq>, cxt: TardisContextExtractor) -> TardisApiResult<i32> {
        let todo_id = TardisFuns::reldb()
            .conn()
            .insert_one(
                todos::ActiveModel {
                    code: Set(todo_add_req.code.to_string()),
                    description: Set(todo_add_req.description.to_string()),
                    done: Set(todo_add_req.done),
                    ..Default::default()
                },
                &cxt.0,
            )
            .await?
            .last_insert_id;
        TardisResp::ok(todo_id)
    }

    // curl -X GET "http://localhost:8089/todo/1" \
    //  -H "Accept: application/json"
    #[oai(path = "/:id", method = "get")]
    async fn get(&self, id: Path<i32>) -> TardisApiResult<TodoDetailResp> {
        let todo = TardisFuns::reldb()
            .conn()
            .get_dto(
                DbQuery::select()
                    .columns(vec![todos::Column::Id, todos::Column::Code, todos::Column::Description, todos::Column::Done])
                    .from(todos::Entity)
                    .and_where(todos::Column::Id.eq(id.0)),
            )
            .await?
            .ok_or_else(|| TardisError::NotFound("Not found".to_string()))?;
        TardisResp::ok(todo)
    }

    #[oai(path = "/", method = "get")]
    async fn get_all(&self, page_number: Query<u64>, page_size: Query<u64>) -> TardisApiResult<TardisPage<TodoDetailResp>> {
        let (todos, total_size) = TardisFuns::reldb()
            .conn()
            .paginate_dtos(
                DbQuery::select().columns(vec![todos::Column::Id, todos::Column::Code, todos::Column::Description, todos::Column::Done]).from(todos::Entity),
                page_number.0,
                page_size.0,
            )
            .await?;
        TardisResp::ok(TardisPage {
            page_number: page_number.0,
            page_size: page_size.0,
            total_size,
            records: todos,
        })
    }

    #[oai(path = "/:id", method = "delete")]
    async fn delete(&self, id: Path<i32>) -> TardisApiResult<u64> {
        let delete_num = TardisFuns::reldb().conn().soft_delete(todos::Entity::find().filter(todos::Column::Id.eq(id.0)), "").await?;
        TardisResp::ok(delete_num)
    }

    // curl -X PUT "http://localhost:8089/todo/1" \
    //  -H "Accept: application/json" \
    //  -H "Tardis-Context: eyJhcHBfaWQiOiAiIiwidGVuYW50X2lkIjogIiIsImFrIjogIiIsImFjY291bnRfaWQiOiAiIiwidG9rZW4iOiAiIiwidG9rZW5fa2luZCI6ICIiLCJyb2xlcyI6IFtdLCJncm91cHMiOiBbXX0=" \
    //  -H "Content-Type: application/json" \
    //  -d '{"description":"AAAAAAAA","done":false}'
    #[oai(path = "/:id", method = "put")]
    async fn update(&self, id: Path<i32>, todo_modify_req: Json<TodoModifyReq>, cxt: TardisContextExtractor) -> TardisApiResult<Void> {
        TardisFuns::reldb()
            .conn()
            .update_one(
                todos::ActiveModel {
                    id: Set(id.0),
                    description: todo_modify_req.description.as_ref().map(|v| Set(v.clone())).unwrap_or(NotSet),
                    done: todo_modify_req.done.map(Set).unwrap_or(NotSet),
                    ..Default::default()
                },
                &cxt.0,
            )
            .await?;
        TardisResp::ok(Void {})
    }
}
