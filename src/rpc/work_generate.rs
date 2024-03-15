use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct WorkGenerateRequest {
    hash: String,
}

#[derive(Serialize)]
pub struct WorkGenerateResponse {
    work: String,
    difficulty: String,
    multiplier: String,
    hash: String,
}

pub(crate) fn on_work_generate(req: WorkGenerateRequest) -> WorkGenerateResponse {
    WorkGenerateResponse {
        work: "2b3d689bbcb21dca".to_string(),
        difficulty: "fffffff93c41ec94".to_string(),
        multiplier: "1.182623871097636".to_string(),
        hash: req.hash,
    }
}
