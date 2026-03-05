use anyhow::{Context, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use esp_idf_hal::modem::Modem;
use std::net::Ipv4Addr;

pub struct WifiStatus {
    pub connected: bool,
    pub ip: Option<Ipv4Addr>,
    pub ssid: Option<String>,
    pub rssi: Option<i8>,
}

pub struct WifiManager<'d> {
    wifi: BlockingWifi<EspWifi<'d>>,
    ssid: String,
}

impl<'d> WifiManager<'d> {
    pub fn new(
        modem: Modem,
        sysloop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        let wifi = EspWifi::new(modem, sysloop.clone(), nvs)
            .context("Failed to create WiFi driver")?;

        let blocking_wifi = BlockingWifi::wrap(wifi, sysloop)
            .context("Failed to create blocking WiFi")?;

        Ok(Self {
            wifi: blocking_wifi,
            ssid: String::new(),
        })
    }

    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        self.ssid = ssid.to_string();

        let config = Configuration::Client(ClientConfiguration {
            ssid: ssid.try_into().map_err(|_| anyhow::anyhow!("SSID too long"))?,
            password: password.try_into().map_err(|_| anyhow::anyhow!("Password too long"))?,
            ..Default::default()
        });

        self.wifi.set_configuration(&config)
            .context("Failed to set WiFi configuration")?;

        self.wifi.start()
            .context("Failed to start WiFi")?;

        log::info!("WiFi started, connecting to '{}'...", ssid);

        self.wifi.connect()
            .context("Failed to connect to WiFi")?;

        self.wifi.wait_netif_up()
            .context("Failed to wait for network interface")?;

        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()
            .context("Failed to get IP info")?;

        log::info!("WiFi connected! IP: {}", ip_info.ip);

        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.wifi.disconnect()
            .context("Failed to disconnect WiFi")?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    pub fn status(&self) -> WifiStatus {
        let connected = self.is_connected();
        let ip = if connected {
            self.wifi.wifi().sta_netif().get_ip_info().ok().map(|info| info.ip)
        } else {
            None
        };

        WifiStatus {
            connected,
            ip,
            ssid: if connected { Some(self.ssid.clone()) } else { None },
            rssi: None,
        }
    }

    pub fn reconnect(&mut self) -> Result<()> {
        if !self.is_connected() {
            self.wifi.connect()
                .context("Failed to reconnect")?;
            self.wifi.wait_netif_up()
                .context("Failed to wait for network interface")?;
        }
        Ok(())
    }
}
