use tabled::Tabled;

#[derive(Tabled)]
pub struct RunnerRow {
    #[tabled(rename = "Name")]
    pub(crate) name: String,
    #[tabled(rename = "Status")]
    pub(crate) status: String,
    #[tabled(rename = "Uptime (s)")]
    pub(crate) uptime: String,
    #[tabled(rename = "Instances")]
    pub(crate) instances: String,
}

#[derive(Tabled)]
pub struct RunnerInstanceRow {
    #[tabled(rename = "Instance ID")]
    pub(crate) id: String,
    #[tabled(rename = "Status")]
    pub(crate) status: String,
    #[tabled(rename = "Uptime (s)")]
    pub(crate) uptime: String,
    #[tabled(rename = "CPU Usage")]
    pub(crate) cpu: String,
    #[tabled(rename = "RAM Usage")]
    pub(crate) ram: String,
    #[tabled(rename = "Data In")]
    pub(crate) rx: String,
    #[tabled(rename = "Data Out")]
    pub(crate) tx: String,
    #[tabled(rename = "Logs")]
    pub(crate) log_len: String,
}

#[derive(Tabled)]
pub struct NodeRow {
    #[tabled(rename = "Node ID")]
    pub(crate) id: String,
    #[tabled(rename = "Status")]
    pub(crate) status: String,
    #[tabled(rename = "IP Address")]
    pub(crate) ip: String,
    #[tabled(rename = "Runners")]
    pub(crate) runner_count: String,
    #[tabled(rename = "Last Updated")]
    pub(crate) updated: String,
}

#[derive(Tabled)]
pub struct UsageRow {
    #[tabled(rename = "Runner ID")]
    pub runner_id: String,
    #[tabled(rename = "Instance ID")]
    pub instance_id: String,
    #[tabled(rename = "Total CPU Time")]
    pub total_cpu: String,
    #[tabled(rename = "Peak CPU")]
    pub peak_cpu: String,
    #[tabled(rename = "Avg RAM")]
    pub avg_ram: String,
    #[tabled(rename = "Peak RAM")]
    pub peak_ram: String,
    #[tabled(rename = "Data In")]
    pub rx: String,
    #[tabled(rename = "Data Out")]
    pub tx: String,
    #[tabled(rename = "Samples")]
    pub samples: String,
}

#[derive(Tabled)]
pub struct BillingEntry {
    #[tabled(rename = "Cost Type")]
    pub label: String,
    #[tabled(rename = "Amount")]
    pub value: String,
}

#[derive(Tabled)]
pub struct GenericRow {
    #[tabled(rename = "Field")]
    pub key: String,
    #[tabled(rename = "Value")]
    pub value: String,
}

#[derive(Tabled)]
pub struct NodeSummaryRow {
    #[tabled(rename = "Node ID")]
    pub node_id: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Client Apps")]
    pub client_apps: u32,
    #[tabled(rename = "System Apps")]
    pub system_apps: u32,
    #[tabled(rename = "Hostname")]
    pub hostname: String,
    #[tabled(rename = "IP Address")]
    pub ip_address: String,
    #[tabled(rename = "Runner Errors")]
    pub warnings: u32,
    #[tabled(rename = "Last Updated")]
    pub last_updated: String,
}
