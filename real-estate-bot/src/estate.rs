use async_trait::async_trait;
use futures::{
    stream::{self, StreamExt},
    FutureExt,
};
use std::{ops::Deref, sync::Arc};

// https://m.land.naver.com/map/getRegionList?cortarNo=1168000000&mycortarNo=
#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionListResponse {
    pub result: RegionListResult,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionListResult {
    pub list: Vec<Region>,
    pub dvsn_info: Option<Region>,
    pub city_info: Option<Region>,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Region {
    #[serde(rename = "CortarNo")]
    pub cortar_no: String,
    #[serde(rename = "CortarNm")]
    pub cortar_nm: String,
    #[serde(rename = "MapXCrdn")]
    pub map_xcrdn: String,
    #[serde(rename = "MapYCrdn")]
    pub map_ycrdn: String,
    #[serde(rename = "CortarType")]
    pub cortar_type: String,
}

// https://m.land.naver.com/complex/ajax/complexListByCortarNo?cortarNo=1168010300
#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexListResponse {
    pub result: Vec<Complex>,
    pub sec_info: Region,
    pub dvsn_info: Region,
    #[serde(rename = "loginYN")]
    pub login_yn: String,
    pub city_info: Region,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Complex {
    pub hscp_no: String,
    pub hscp_nm: String,
    pub hscp_type_cd: String,
    pub hscp_type_nm: String,
    pub lat: String,
    pub lng: String,
    pub cortar_no: String,
    pub deal_cnt: i64,
    pub lease_cnt: i64,
    pub rent_cnt: i64,
    pub strm_rent_cnt: i64,
    pub has_book_mark: i64,
}

// https://m.land.naver.com/complex/getComplexArticleList?hscpNo=8928&tradTpCd=A1&order=price&showR0=N&page=1
#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexArticleListResponse {
    pub result: ComplexArticleListResult,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexArticleListResult {
    pub list: Vec<ComplexArticle>,
    pub tot_atcl_cnt: i64,
    pub more_data_yn: String,
    pub show_guarantee: bool,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexArticle {
    pub rep_img_url: Option<String>,
    pub atcl_no: Option<String>,
    pub rep_img_tp_cd: Option<String>,
    pub vrfc_tp_cd: Option<String>,
    pub atcl_nm: Option<String>,
    pub bild_nm: Option<String>,
    pub trad_tp_cd: Option<String>,
    pub trad_tp_nm: Option<String>,
    pub rlet_tp_cd: Option<String>,
    pub rlet_tp_nm: Option<String>,
    pub spc1: Option<String>,
    pub spc2: Option<String>,
    pub flr_info: Option<String>,
    pub atcl_fetr_desc: Option<String>,
    pub cfm_ymd: Option<String>,
    pub prc_info: Option<String>,
    pub same_addr_cnt: i64,
    pub same_addr_direct_cnt: i64,
    pub same_addr_hash: Option<String>,
    pub same_addr_max_prc: Option<String>,
    pub same_addr_min_prc: Option<String>,
    pub trad_cmpl_yn: Option<String>,
    pub tag_list: Vec<Option<String>>,
    pub atcl_stat_cd: Option<String>,
    pub cpid: Option<String>,
    pub cp_nm: Option<String>,
    pub cp_cnt: i64,
    pub rltr_nm: Option<String>,
    pub direct_trad_yn: Option<String>,
    pub direction: Option<String>,
    pub trade_price_han: Option<String>,
    pub trade_rent_price: i64,
    pub trade_price_info: Option<String>,
    pub trade_checked_by_owner: bool,
    pub point: i64,
    pub dtl_addr: Option<String>,
    pub dtl_addr_yn: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    SerdeError(#[from] serde_json::error::Error),
}

fn result_flatten<T, E: From<E1>, E1>(r: Result<Result<T, E>, E1>) -> Result<T, E> {
    r.map_err(|e| e.into()).and_then(|y| y)
}

#[async_trait]
pub trait EstateService: Send + Sync + Sized + Clone + 'static {
    async fn region_list(self: Arc<Self>, parent_region_id: String) -> Result<Vec<Region>, Error>;
    async fn complex_list(self: Arc<Self>, region_id: String) -> Result<Vec<Complex>, Error>;
    async fn complex_article_list(
        self: Arc<Self>,
        complex_id: String,
    ) -> Result<Vec<ComplexArticle>, Error>;
}

#[derive(Debug, Clone)]
pub struct EstateServiceLive;

const BASE_URL: &str = "https://m.land.naver.com";

#[async_trait]
impl EstateService for EstateServiceLive {
    async fn region_list(self: Arc<Self>, parent_region_id: String) -> Result<Vec<Region>, Error> {
        let url = format!(
            "{}/map/getRegionList?cortarNo={}",
            BASE_URL, parent_region_id
        );
        let resp = reqwest::get(&url).await?;
        let resp_text = resp.text().await?;
        let result: RegionListResponse = serde_json::from_str(resp_text.as_str())?;
        Ok(result.result.list)
    }

    async fn complex_list(self: Arc<Self>, region_id: String) -> Result<Vec<Complex>, Error> {
        let url = format!(
            "{}/complex/ajax/complexListByCortarNo?cortarNo={}",
            BASE_URL, region_id
        );
        let resp = reqwest::get(&url).await?;
        let result: ComplexListResponse = serde_json::from_str(resp.text().await?.as_str())?;
        Ok(result.result)
    }

    async fn complex_article_list(
        self: Arc<Self>,
        complex_id: String,
    ) -> Result<Vec<ComplexArticle>, Error> {
        let mut page = 1;
        let mut rs: Vec<ComplexArticle> = Vec::new();
        loop {
            let url = format!("{}/complex/getComplexArticleList?hscpNo={}&tradTpCd=A1&order=price&showR0=N&page={}", BASE_URL, complex_id, page);
            let resp = reqwest::get(&url).await?;
            let resp_text = resp.text().await?;
            println!("{}", resp_text);
            let result: ComplexArticleListResponse = serde_json::from_str(resp_text.as_str())?;
            rs.extend(result.result.list);
            if !result.result.more_data_yn.eq_ignore_ascii_case("Y") {
                break;
            } else {
                page += 1;
            }
        }
        Ok(rs)
    }
}
