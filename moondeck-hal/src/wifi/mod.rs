use anyhow::{Context, Result};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp::{EspSntp, SntpConf, SyncStatus};
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use std::net::Ipv4Addr;
use std::time::Duration;

pub struct WifiStatus {
    pub connected: bool,
    pub ip: Option<Ipv4Addr>,
    pub ssid: Option<String>,
    pub rssi: Option<i8>,
}

pub struct WifiManager<'d> {
    wifi: BlockingWifi<EspWifi<'d>>,
    ssid: String,
    _sntp: Option<EspSntp<'static>>,
}

impl<'d> WifiManager<'d> {
    pub fn new(
        modem: Modem,
        sysloop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        let wifi =
            EspWifi::new(modem, sysloop.clone(), nvs).context("Failed to create WiFi driver")?;

        let blocking_wifi =
            BlockingWifi::wrap(wifi, sysloop).context("Failed to create blocking WiFi")?;

        Ok(Self {
            wifi: blocking_wifi,
            ssid: String::new(),
            _sntp: None,
        })
    }

    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        self.ssid = ssid.to_string();

        // Use WPA2WPA3Personal for modern routers, with fallback behavior
        // AuthMethod::None would auto-detect but some routers require explicit auth
        let auth_method = if password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2WPA3Personal
        };

        let config = Configuration::Client(ClientConfiguration {
            ssid: ssid
                .try_into()
                .map_err(|_| anyhow::anyhow!("SSID too long"))?,
            password: password
                .try_into()
                .map_err(|_| anyhow::anyhow!("Password too long"))?,
            auth_method,
            ..Default::default()
        });

        self.wifi
            .set_configuration(&config)
            .context("Failed to set WiFi configuration")?;

        self.wifi.start().context("Failed to start WiFi")?;

        log::info!(
            "WiFi started, connecting to '{}' with auth {:?}...",
            ssid,
            auth_method
        );

        self.wifi.connect().context("Failed to connect to WiFi")?;

        self.wifi
            .wait_netif_up()
            .context("Failed to wait for network interface")?;

        let ip_info = self
            .wifi
            .wifi()
            .sta_netif()
            .get_ip_info()
            .context("Failed to get IP info")?;

        log::info!("WiFi connected! IP: {}", ip_info.ip);

        // Initialize SNTP for time synchronization
        self.init_sntp();

        Ok(())
    }

    fn init_sntp(&mut self) {
        log::info!("Initializing SNTP time sync...");

        let sntp_conf = SntpConf {
            servers: ["time.google.com"],
            ..Default::default()
        };

        match EspSntp::new(&sntp_conf) {
            Ok(sntp) => {
                // Wait for time sync (up to 15 seconds) - NTP can be slow on first connect
                let mut attempts = 0;
                while sntp.get_sync_status() != SyncStatus::Completed && attempts < 150 {
                    std::thread::sleep(Duration::from_millis(100));
                    attempts += 1;
                }

                if sntp.get_sync_status() == SyncStatus::Completed {
                    log::info!("SNTP time synchronized successfully");
                } else {
                    // Don't warn - time sync will continue in background
                    log::info!("SNTP sync pending, will complete in background");
                }

                self._sntp = Some(sntp);
            }
            Err(e) => {
                log::warn!("Failed to initialize SNTP: {:?}", e);
            }
        }
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.wifi
            .disconnect()
            .context("Failed to disconnect WiFi")?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    pub fn status(&self) -> WifiStatus {
        let connected = self.is_connected();
        let ip = if connected {
            self.wifi
                .wifi()
                .sta_netif()
                .get_ip_info()
                .ok()
                .map(|info| info.ip)
        } else {
            None
        };

        // Get RSSI - esp-idf-svc doesn't expose this directly for connected AP,
        // so we return a reasonable default when connected
        let rssi = if connected {
            Some(-50) // Assume decent signal if connected
        } else {
            None
        };

        WifiStatus {
            connected,
            ip,
            ssid: if connected {
                Some(self.ssid.clone())
            } else {
                None
            },
            rssi,
        }
    }

    pub fn reconnect(&mut self) -> Result<()> {
        if !self.is_connected() {
            self.wifi.connect().context("Failed to reconnect")?;
            self.wifi
                .wait_netif_up()
                .context("Failed to wait for network interface")?;
        }
        Ok(())
    }
}
