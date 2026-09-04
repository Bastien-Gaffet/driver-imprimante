use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub struct DiscoveredPrinter {
    pub ip: String,
    pub name: String,
}

pub fn printer_from_service_info(info: &ResolvedService) -> Option<DiscoveredPrinter> {
    let ip = info.get_addresses_v4().into_iter().next()?.to_string();
    let fullname = info.get_fullname();
    let name = fullname.split("._").next().unwrap_or(fullname).to_string();
    Some(DiscoveredPrinter { ip, name })
}

pub fn discover(timeout: Duration) -> mdns_sd::Result<Option<DiscoveredPrinter>> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse("_ipp._tcp.local.")?;
    let deadline = Instant::now() + timeout;

    let found = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break None;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(printer) = printer_from_service_info(&info) {
                    break Some(printer);
                }
            }
            Ok(_) => continue,
            Err(_) => break None,
        }
    };

    mdns.shutdown()?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdns_sd::ServiceInfo;

    #[test]
    fn extracts_ip_and_name_from_service_info() {
        let info = ServiceInfo::new(
            "_ipp._tcp.local.",
            "Brother HL-L3240CDW",
            "brother-printer.local.",
            "192.168.1.50",
            631u16,
            None,
        )
        .unwrap()
        .as_resolved_service();

        let printer = printer_from_service_info(&info).unwrap();

        assert_eq!(printer.ip, "192.168.1.50");
        assert_eq!(printer.name, "Brother HL-L3240CDW");
    }
}
