//extern crate hyper;
//extern crate hyper_rustls;
extern crate google_sheets4 as sheets4;

use std::sync::{Arc, Mutex};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use sheets4::api::ValueRange;
//use sheets4::{Result, Error};
use std::default::Default;
use sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use thiserror::Error;
//use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
use crate::gsheets::oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
//use serde::de::DeserializeOwned;
use lazy_static::lazy_static;
use regex::Regex;
use std::path::PathBuf;
use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use crate::dcare_order::{
    OrderNew,
    OrderUpdate,
};

#[derive(Error, Debug)]
pub enum SheetsError {
    #[error("SERVICE_ACCOUNT_JSON not defined")]
    EnvVarNotFound(#[from] std::env::VarError),

    #[error("Invalid service account JSON")]
    InvalidServiceAccountJSON(#[from] serde_json::Error),

    #[error("Error with token cache path")]
    TokenCachePathError(#[from] std::io::Error),

    #[error(transparent)]
    SheetsError(#[from] google_sheets4::Error),

    //#[error(transparent)]
    //CSVError(#[from] csv::Error),
    #[error("Internal error")]
    InternalUTFError(#[from] std::string::FromUtf8Error),

    //#[error("Internal error")]
    //InternalWriterError(#[from] csv::IntoInnerError<Writer<Vec<u8>>>),
    #[error("Update Range format invalid")]
    UpdateRangeError,
}

fn column_shift(column: &str, num: u32) -> String {
    let mut is_last = true;
    let last: String = column
        .chars()
        .rev()
        .map(|c| {
            if is_last {
                is_last = false;
                let b: u32 = u32::from(c) + num;
                char::from_u32(b).unwrap()
            } else {
                c
            }
        })
        .collect();

    last.chars().rev().collect()
}

#[derive(Debug, Default)]
pub struct GooglesheetPosition {
    pub column: String, /* A ~ ...*/
    pub row: i32,
}

impl GooglesheetPosition {
    fn parse(n1: String) -> Result<Self, SheetsError> {
        /* ex, "'工單表'!D24:E24" */
        lazy_static! {
            static ref RE: Regex = Regex::new(r"[^!]+!([A-Z]+)(\d+):.+$").unwrap();
        }

        if let Some(cap) = RE.captures(&n1) {
            //println!("[GooglesheetPosition:from] {n1} => {} as {} + {}\n", &cap[0], &cap[1], &cap[2]);

            Ok(Self {
                column: cap[1].to_string(),
                row: cap[2].parse::<i32>().unwrap(),
            })
        } else {
            Err(SheetsError::UpdateRangeError)
        }
    }

    #[allow(dead_code)]
    fn shift(&mut self, num: u32) -> Result<(), SheetsError> {
        self.column = column_shift(&self.column, num);

        Ok(())
    }

    fn generate(&self, tab_name: &str, data_num: u32) -> Result<String, SheetsError> {
        let last = column_shift(&self.column, data_num);

        Ok(format!(
            "\'{}\'!{}{}:{}{}",
            tab_name, self.column, self.row, last, self.row
        ))
    }
}

#[derive(Clone)]
pub struct SharedDcareGoogleSheet {
    document_id: String,
    tab_name: String,
    inner: Arc<Mutex<DcareGoogleSheet>>,
}

impl SharedDcareGoogleSheet {
    pub async fn new(
        key: Option<String>,
        document_id: &str,
        tab_name: &str,
    ) -> Result<Self, SheetsError> {
        DcareGoogleSheet::new(key)
            .await
            .map(|g| Self {
                document_id: document_id.to_string(),
                tab_name: tab_name.to_string(),
                inner: Arc::new(Mutex::new(g))
            })
    }

    fn get_sheets(&self) -> Sheets<HttpsConnector<HttpConnector>> {
        let lock = self.inner.lock().unwrap();
        lock.sheets.clone()
    }

    /*pub async fn append_order(
        &self,
        order: &OrderNew,
        order_id: i32,
        issue_at: DateTime<Utc>,
        sn: &str
    ) -> Result<GooglesheetPosition, SheetsError> {
        let sheet = self.get_sheets();


        /* TODO */
        Err(SheetsError::UpdateRangeError)
    }*/

    #[allow(dead_code)]
    pub async fn append(
        &self,
        data: Vec<String>
    ) -> Result<GooglesheetPosition, SheetsError> {
        let reqs: Vec<Vec<String>> = vec![data];

        let req = ValueRange {
            major_dimension: None,
            range: Some(self.tab_name.clone()),
            values: Some(reqs),
        };

        let sheets = self.get_sheets();
        let res = sheets
            .spreadsheets()
            .values_append(req, &self.document_id, &self.tab_name)
            .value_input_option("USER_ENTERED")
            .include_values_in_response(false)
            .doit()
            .await;

        //println!("sheet values_append return {:?}", res);

        res.ok()
            .and_then(|(_, appended)| {
                appended
                    .updates
                    .and_then(|u| u.updated_range.map(GooglesheetPosition::parse))
            })
            .unwrap_or(Err(SheetsError::UpdateRangeError))
    }

    #[allow(dead_code)]
    pub async fn modify(
        &self,
        data: Vec<String>,
        position: GooglesheetPosition,
    ) -> Result<(), SheetsError> {
        let num = data.len() as u32;
        let reqs: Vec<Vec<String>> = vec![data];

        let set_range = position.generate(&self.tab_name, num - 1)?;
        //println!("[debug][modify] set-range = {:?}", set_range);

        let req = ValueRange {
            major_dimension: None,
            range: Some(set_range.clone()),
            values: Some(reqs),
        };

        let sheets = self.get_sheets();
        let _res = sheets
            .spreadsheets()
            .values_update(req, &self.document_id, &set_range)
            .value_input_option("USER_ENTERED")
            .include_values_in_response(false)
            .doit()
            .await?;

        //println!("sheet values_append return {:?}", _res);

        Ok(())
        /*let got_range = res
            .ok()
            .and_then(|(_, updated)| updated
                      .updated_range
                      .map(GooglesheetPosition::from));


        range.ok_or(SheetsError::UpdateRangeError)*/
    }
}

struct DcareGoogleSheet {
    sheets: Sheets<HttpsConnector<HttpConnector>>,
}

impl DcareGoogleSheet {
    #[allow(dead_code)]
    fn service_account_from(
        key: Option<String>,
    ) -> Result<ServiceAccountKey, SheetsError> {
        let key = if let Some(k) = key {
            k
        } else {
            std::env::var("SERVICE_ACCOUNT_JSON")?
        };
        let key = serde_json::from_str(&key)?;

        Ok(key)
    }

    #[allow(dead_code)]
    async fn new/*<P: Into<PathBuf>>*/(
        key: Option<String>,
    ) -> Result<Self, SheetsError> {
        let service_account = Self::service_account_from(key)?;

        let builder = ServiceAccountAuthenticator::builder(service_account);
        /*let auth = if let Some(path) = token_cache_path {
            builder.persist_tokens_to_disk(path).build().await?
        } else {
            builder.build().await?
        };*/
        let auth = builder.build().await?;
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http1()
            .enable_http2()
            .build();
        let sheets = Sheets::new(hyper::Client::builder().build(connector), auth);

        Ok(DcareGoogleSheet {
            sheets,
        })
    }

    /*#[allow(dead_code)]
    async fn append_last(
        &self,
        document_id: &str,
        tab_name: &str,
        data: Vec<String>
    ) -> Result<GooglesheetPosition, SheetsError> {
        let reqs: Vec<Vec<String>> = vec![data];

        let req = ValueRange {
            major_dimension: None,
            range: Some(tab_name.to_string()),
            values: Some(reqs),
        };

        let res = self
            .sheets
            .spreadsheets()
            .values_append(req, document_id, tab_name)
            .value_input_option("USER_ENTERED")
            .include_values_in_response(false)
            .doit()
            .await;

        //println!("sheet values_append return {:?}", res);

        res.ok()
            .and_then(|(_, appended)| {
                appended
                    .updates
                    .and_then(|u| u.updated_range.map(GooglesheetPosition::parse))
            })
            .unwrap_or(Err(SheetsError::UpdateRangeError))
    }

    #[allow(dead_code)]
    pub async fn modify(
        &self,
        document_id: &str,
        tab_name: &str,
        data: Vec<String>,
        position: GooglesheetPosition,
    ) -> Result<(), SheetsError> {
        let num = data.len() as u32;
        let reqs: Vec<Vec<String>> = vec![data];

        let set_range = position.generate(tab_name, num - 1)?;
        //println!("[debug][modify] set-range = {:?}", set_range);

        let req = ValueRange {
            major_dimension: None,
            range: Some(set_range.clone()),
            values: Some(reqs),
        };

        let _res = self
            .sheets
            .spreadsheets()
            .values_update(req, document_id, &set_range)
            .value_input_option("USER_ENTERED")
            .include_values_in_response(false)
            .doit()
            .await?;

        //println!("sheet values_append return {:?}", _res);

        Ok(())
        /*let got_range = res
            .ok()
            .and_then(|(_, updated)| updated
                      .updated_range
                      .map(GooglesheetPosition::from));


        range.ok_or(SheetsError::UpdateRangeError)*/
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_account_from_env() {
        /* export SERVICE_ACCOUNT_JSON=(cat gsheet_test/proven-space-378115-cfa4fbd08e11.json | jq -c) */
        let key = DcareGoogleSheet::service_account_from_env();

        assert!(key.is_ok())
    }

    async fn try_create() -> Result<SharedDcareGoogleSheet, SheetsError> {
        SharedDcareGoogleSheet::new(
            "19cQ_zAgqkM_iqOiqECP1yVTobuRkFbwk-VfegOys8ZE",
            "工單表",
        )
        .await
    }

    #[tokio::test]
    async fn test_create() {
        let gsheet = try_create().await;

        assert!(gsheet.is_ok())
    }

    #[tokio::test]
    async fn test_append() {
        let req = vec![
            "2023/1/12 下午 10:10:26".to_string(),
            "DB2301122210300".to_string(),
            "斗六BM店（05-5372527）".to_string(),
            "開單人員1".to_string(),
            "孔小姐".to_string(),
            "0911123456".to_string(),
            "斗六市成功二街13號".to_string(),
            "Dyson".to_string(),
            "V11".to_string(),
            "製2019".to_string(),
            "主機".to_string(),
            "電動刷頭".to_string(),
            "共2個刷頭".to_string(),
            "污損.to_string(), 刮痕.to_string(), 掉漆".to_string(),
            "清潔.維修".to_string(),
            "B-啟動有異音".to_string(),
            "".to_string(),
            " 馬達有問題，會發出咚的一聲".to_string(),
            "".to_string(),
            "全新馬達58 主機+大刷頭+小 刷頭\n =75元 1/13".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "資料入檔中".to_string(),
            "未結清".to_string(),
            "報價中".to_string(),
        ];

        let gsheet = try_create().await.unwrap();

        let res = gsheet.append(req).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_modify() {
        let gsheet = try_create().await.unwrap();

        let req = vec!["111111".to_string(), "222222".to_string()];
        let pos = GooglesheetPosition {
            column: "F".to_string(),
            row: 32,
        };

        let res = gsheet.modify(req, pos).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_new2change() {
        let req = vec![
            "2023/1/12 下午 10:10:26".to_string(),
            "DB2301122210399".to_string(),
            "斗六BM店".to_string(),
            "小姐1".to_string(),
            "小姐2".to_string(),
            "0911123456".to_string(),
            "斗六市成功二街13號".to_string(),
            "roomba".to_string(),
            "V11".to_string(),
            "2021".to_string(),
            "主機".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "11111".to_string(),
            "22222".to_string(),
        ];

        let gsheet = try_create().await.unwrap();

        let res = gsheet.append(req).await;
        assert!(res.is_ok());

        //println!("[debug] res = {:?}", res);

        let req = vec!["未結清".to_string(), "報價中".to_string()];
        let mut pos = res.unwrap();
        let _ = pos.shift(24);
        //println!("[debug] new post = {:?}", pos);

        let res = gsheet.modify(req, pos).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_chars() {
        let a = 'A';

        let b: u32 = u32::from(a) + 1;
        let b = char::from_u32(b);

        assert_eq!(b, Some('B'));

        let aa = "AB";

        let mut is_last = true;
        let last: String = aa
            .chars()
            .rev()
            .map(|c| {
                if is_last {
                    is_last = false;
                    let b: u32 = u32::from(c) + 1;
                    char::from_u32(b).unwrap()
                } else {
                    c
                }
            })
            .collect();

        assert_eq!(last, "CA");

        let last: String = last.chars().rev().collect();

        assert_eq!(last, "AC");
    }
}
