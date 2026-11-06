## ADDED Requirements

### Requirement: Virtual Network Interface & DHCP
The firmware SHALL establish an asynchronous IP network over USB CDC-NCM with a static IP and DHCP server.

#### Scenario: Host IP acquisition
- **WHEN** the host OS recognizes the CDC-NCM Ethernet adapter
- **THEN** the firmware's DHCP server assigns an IP address in the `10.10.10.0/24` subnet and sets `10.10.10.10` as the DNS and gateway

### Requirement: Captive Portal DNS
The firmware SHALL intercept DNS queries and resolve `agentla.local` to `10.10.10.10`.

#### Scenario: Resolving agentla.local
- **WHEN** a browser sends an A record DNS query for `agentla.local`
- **THEN** the embedded DNS responder returns IP `10.10.10.10`

### Requirement: Embedded Asynchronous HTTP & REST Server
The firmware SHALL run `picoserve` to serve web assets from LittleFS and handle REST API endpoints.

#### Scenario: Serving index.html from LittleFS
- **WHEN** an HTTP GET request is received for `/` or `/index.html`
- **THEN** `picoserve` reads the file from LittleFS and streams the HTML with `Content-Type: text/html`

#### Scenario: System info REST endpoint
- **WHEN** an HTTP GET request is received for `/api/v1/system`
- **THEN** `picoserve` returns JSON containing board architecture, clock speed, uptime, and free RAM
