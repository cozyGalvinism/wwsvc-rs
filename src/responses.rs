use serde::Deserialize;

use crate::generate_get_response;

/// COMRESULT of a request. Contains information about the status of the request.
#[derive(Deserialize, Debug, Clone)]
pub struct ComResult {
    /// The HTTP status code of the request.
    #[serde(rename = "STATUS")]
    pub status: u32,
    /// The HTTP status message of the request.
    #[serde(rename = "CODE")]
    pub code: String,
    /// Information about the request.
    #[serde(rename = "INFO")]
    pub info: String,
    /// Additional information about the request.
    #[serde(rename = "INFO2")]
    pub info2: Option<String>,
    /// Additional information about the request.
    #[serde(rename = "INFO3")]
    pub info3: Option<String>,
    /// Error number of the request.
    #[serde(rename = "ERRNO")]
    pub errno: Option<String>,
}

/// Response of a REGISTER request.
#[derive(Deserialize, Clone)]
pub struct RegisterResponse {
    /// The COMRESULT of the request. Contains information about the status of the request.
    #[serde(rename = "COMRESULT")]
    pub com_result: ComResult,
    /// The returned service pass.
    #[serde(rename = "SERVICEPASS")]
    pub service_pass: ServicePass,
}

/// Service pass of a REGISTER request.
#[derive(Deserialize, Clone)]
pub struct ServicePass {
    /// The service pass.
    #[serde(rename = "PASSID")]
    pub pass_id: String,
    /// The application ID linked to the service pass.
    #[serde(rename = "APPID")]
    pub app_id: String,
}

generate_get_response!(ArtikelGetResponse, "ARTIKELLISTE", ArtikelListe, "ARTIKEL");
generate_get_response!(AdresseGetResponse, "ADRESSLISTE", AdresseListe, "ADRESSE");
generate_get_response!(BelegGetResponse, "BELEGLISTE", BelegListe, "BELEG");
generate_get_response!(
    BelPosGetResponse,
    "POSITIONSLISTE",
    PositionListe,
    "POSITION"
);
generate_get_response!(ProjektGetResponse, "PROJEKTLISTE", ProjektListe, "PROJEKT");
generate_get_response!(
    SeriennummerGetResponse,
    "SERIENNUMMERNLISTE",
    SeriennummerListe,
    "SERIENNUMMER"
);
generate_get_response!(ChargeGetResponse, "CHARGENLISTE", ChargeListe, "CHARGE");
generate_get_response!(
    AdressArtikelGetResponse,
    "ADRESSARTIKELLISTE",
    AdressArtikelListe,
    "ADRESSARTIKEL"
);
generate_get_response!(
    LieferadresseGetResponse,
    "LIEFERADRESSLISTE",
    LieferadresseListe,
    "LIEFERADRESSE"
);
generate_get_response!(
    AnsprechpartnerGetResponse,
    "ANSPRECHPARTNERLISTE",
    AnsprechpartnerListe,
    "ANSPRECHPARTNER"
);
generate_get_response!(
    VertreterGetResponse,
    "VERTRETERLISTE",
    VertreterListe,
    "VERTRETER"
);
generate_get_response!(TermineGetResponse, "TERMINLISTE", TerminListe, "TERMIN");
generate_get_response!(
    GespraechGetResponse,
    "GESPRAECHELISTE",
    GespraechListe,
    "GESPRAECH"
);
generate_get_response!(
    WiedervorlageGetResponse,
    "WIEDERVORLAGELISTE",
    WiedervorlageListe,
    "WIEDERVORLAGE"
);
generate_get_response!(
    WarengruppeGetResponse,
    "WARENGRUPPENLISTE",
    WarengruppeListe,
    "WARENGRUPPE"
);
generate_get_response!(LagerGetResponse, "LAGERLISTE", LagerListe, "LAGER");
generate_get_response!(
    MPKatalogGetResponse,
    "KATALOGLISTE",
    MPKatalogListe,
    "KATALOG"
);
generate_get_response!(
    MPKategorieGetResponse,
    "KATEGORIENLISTE",
    MPKategorieListe,
    "KATEGORIE"
);
generate_get_response!(EANCodeGetResponse, "EANCODELISTE", EANCodeListe, "EANCODE");
