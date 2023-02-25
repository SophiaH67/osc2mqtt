use async_osc::OscType;

pub fn hass_arg_to_osc(arg: String) -> OscType {
    match arg.as_str() {
        "ON" => OscType::Bool(true),
        "OFF" => OscType::Bool(false),
        _ => OscType::Float(arg.parse().unwrap()),
    }
}

pub fn osc_arg_to_hass(osc_arg: &OscType) -> String {
    match osc_arg {
        OscType::Bool(value) => {
            if *value {
                "ON".to_string()
            } else {
                "OFF".to_string()
            }
        }
        OscType::Float(value) => value.to_string(),
        OscType::Int(value) => value.to_string(),
        _ => panic!("Unsupported OSC type"),
    }
}
