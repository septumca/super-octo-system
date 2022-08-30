const PLANET_NAMES: [&str; 63] = [
  "Metis",    "Adrastea",   "Amalthea",   "Thebe",
  "Io",       "Europa",     "Ganymede",   "Callisto",
  "Themisto", "Leda",       "Himalia",    "Lysithea",
  "Elara",    "Dia",        "Carpo",      "S/2003",
  "Euporie",  "S/2003",     "S/2003",     "Thelxinoe",
  "Euanthe",  "Helike",     "Orthosie",   "Iocaste",
  "S/2003",   "Praxidike",  "Harpalyke",  "Mneme",
  "Hermippe", "Thyone",     "Ananke",     "Herse",
  "Aitne",    "Kale",       "Taygete",    "S/2003",
  "Chaldene", "S/2003",     "S/2003",     "S/2003",
  "Erinome",  "Aoede",      "Kallichore", "Kalyke",
  "Carme",    "Callirrhoe", "Eurydome",   "Pasithee",
  "Kore",     "Cyllene",    "Eukelade",   "S/2003",
  "Pasiphaë", "Hegemone",   "Arche",      "Isonoe",
  "S/2003",   "S/2003",     "Sinope",     "Sponde",
  "Autonoe",  "Megaclite",  "S/2003"
];

pub struct NamesGen {
  available_names: Vec<String>
}

impl NamesGen {
  pub fn new() -> Self {
    Self {
      available_names: vec![]
    }
  }
}

