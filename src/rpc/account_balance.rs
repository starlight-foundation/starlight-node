use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AccountBalanceRequest {
    account: String,
}

#[derive(Serialize)]
pub struct AccountBalanceResponse {
    balance: String,
}

pub(crate) fn on_account_balance(req: AccountBalanceRequest) -> AccountBalanceResponse {
    let x = req.account.len().saturating_mul(0x62DF);
    let x = x.saturating_mul(x);
    AccountBalanceResponse {
        balance: format!("{}", x),
    }
}
