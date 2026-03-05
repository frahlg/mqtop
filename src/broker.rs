#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerKind {
    Mqtt,
    Nats,
}

impl BrokerKind {
    pub fn label(&self) -> &'static str {
        match self {
            BrokerKind::Mqtt => "MQTT",
            BrokerKind::Nats => "NATS",
        }
    }

    pub fn topic_separator(&self) -> char {
        match self {
            BrokerKind::Mqtt => '/',
            BrokerKind::Nats => '.',
        }
    }

    pub fn filter_title_hint(&self) -> &'static str {
        match self {
            BrokerKind::Mqtt => "MQTT wildcards: + #",
            BrokerKind::Nats => "NATS wildcards: * >",
        }
    }

    pub fn wildcard_single(&self) -> char {
        match self {
            BrokerKind::Mqtt => '+',
            BrokerKind::Nats => '*',
        }
    }

    pub fn wildcard_multi(&self) -> char {
        match self {
            BrokerKind::Mqtt => '#',
            BrokerKind::Nats => '>',
        }
    }

    pub fn default_subscribe_pattern(&self) -> &'static str {
        match self {
            BrokerKind::Mqtt => "#",
            BrokerKind::Nats => ">",
        }
    }
}
