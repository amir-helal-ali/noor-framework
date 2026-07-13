# Deployment Guide | دليل النشر

## Overview | نظرة عامة

This guide covers deploying Noor Framework applications to various environments.

يغطي هذا الدليل نشر تطبيقات إطار عمل نور في بيئات مختلفة.

## Table of Contents | فهرس المحتويات

1. [Docker Deployment | النشر بـ Docker](#docker-deployment--النشر-بـ-docker)
2. [Manual Deployment | النشر اليدوي](#manual-deployment--النشر-اليدوي)
3. [Nginx Configuration | إعدادات Nginx](#nginx-configuration--إعدادات-nginx)
4. [SSL/HTTPS Setup | إعداد SSL/HTTPS](#sslhttps-setup--إعداد-sslhttps)
5. [Production Checklist | قائمة فحص الإنتاج](#production-checklist--قائمة-فحص-الإنتاج)
6. [Monitoring | المراقبة](#monitoring--المراقبة)
7. [Scaling | التوسع](#scaling--التوسع)

---

## Docker Deployment | النشر بـ Docker

### Quick Start | البداية السريعة

```bash
# Build the image
docker build -t noor-app .

# Run the container
docker run -d \
  --name noor-app \
  -p 8080:8080 \
  -e APP_ENV=production \
  -e JWT_SECRET=your-production-secret \
  -v ./storage:/app/storage \
  noor-app
```

### Docker Compose | Docker Compose

```bash
# Production
docker-compose up -d

# With PostgreSQL and Nginx
docker-compose --profile production up -d

# High-traffic (with Redis)
docker-compose --profile high-traffic up -d
```

### Weak Server Deployment | النشر على السيرفرات الضعيفة

```bash
# Build optimized for weak servers
docker build --target weak-server -t noor:weak .

# Run with minimal resources
docker run -d \
  --memory=128m \
  --cpus=0.25 \
  -p 8080:8080 \
  noor:weak
```

### Environment Variables | متغيرات البيئة

```env
# Application
APP_ENV=production
APP_NAME=My App

# Server
NOOR_SERVER_HOST=0.0.0.0
NOOR_SERVER_PORT=8080

# Database
DATABASE_URL=postgres://user:pass@db:5432/noor

# Security
JWT_SECRET=use-a-strong-random-secret
RUST_LOG=info

# Docker
NOOR_PORT=8080
POSTGRES_PASSWORD=strong-password
```

---

## Manual Deployment | النشر اليدوي

### Prerequisites | المتطلبات

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Zig (optional, for performance modules)
curl -L https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz | tar -xJ -C /usr/local
ln -s /usr/local/zig-linux-x86_64-0.11.0/zig /usr/local/bin/zig
```

### Build | البناء

```bash
# Clone the project
git clone https://github.com/your-org/your-app.git
cd your-app

# Build for production
cargo build --release

# For weak servers (optimized for size)
cargo build --profile weak-server
```

### Run | التشغيل

```bash
# Create a system user
sudo useradd -r -s /bin/false noor

# Create directories
sudo mkdir -p /opt/noor /var/lib/noor/storage
sudo chown -R noor:noor /opt/noor /var/lib/noor

# Copy binary
sudo cp target/release/noor-server /opt/noor/

# Copy config
sudo cp noor.toml /opt/noor/

# Create systemd service
sudo tee /etc/systemd/system/noor.service <<EOF
[Unit]
Description=Noor Framework Application
After=network.target

[Service]
Type=simple
User=noor
Group=noor
WorkingDirectory=/opt/noor
Environment=APP_ENV=production
Environment=NOOR_SERVER_HOST=0.0.0.0
Environment=NOOR_SERVER_PORT=8080
Environment=JWT_SECRET=your-production-secret
Environment=DATABASE_URL=sqlite:///var/lib/noor/storage/noor.db
Environment=RUST_LOG=info
ExecStart=/opt/noor/noor-server
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable noor
sudo systemctl start noor

# Check status
sudo systemctl status noor
```

---

## Nginx Configuration | إعدادات Nginx

```nginx
server {
    listen 80;
    server_name yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com;
    
    # SSL
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    
    # Security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    
    # Upload size
    client_max_body_size 10M;
    
    # Static files
    location /static/ {
        alias /opt/noor/public/;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
    
    # Reverse proxy
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_connect_timeout 5s;
        proxy_read_timeout 30s;
    }
    
    # Health check
    location /health {
        proxy_pass http://127.0.0.1:8080;
        access_log off;
    }
}
```

---

## SSL/HTTPS Setup | إعداد SSL/HTTPS

### Using Let's Encrypt | باستخدام Let's Encrypt

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d yourdomain.com

# Auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

### Using Cloudflare | باستخدام Cloudflare

1. Add your domain to Cloudflare
2. Change nameservers
3. Enable "Full (Strict)" SSL mode
4. Enable "Always Use HTTPS"
5. Enable "HSTS"

---

## Production Checklist | قائمة فحص الإنتاج

### Security | الأمان

- [ ] Set `APP_ENV=production`
- [ ] Set strong `JWT_SECRET` (64+ characters)
- [ ] Set `debug = false`
- [ ] Enable HTTPS
- [ ] Configure CORS origins
- [ ] Enable all security headers
- [ ] Set up firewall (ufw)
- [ ] Disable root SSH login
- [ ] Set up fail2ban

### Performance | الأداء

- [ ] Enable OPcache (if using PHP)
- [ ] Configure cache (file or Redis)
- [ ] Enable gzip compression
- [ ] Set up CDN for static assets
- [ ] Configure database connection pooling
- [ ] Enable query caching

### Reliability | الموثوقية

- [ ] Set up automatic backups
- [ ] Configure health checks
- [ ] Set up monitoring (Prometheus/Grafana)
- [ ] Configure log rotation
- [ ] Set up error tracking (Sentry)
- [ ] Configure uptime monitoring

### Database | قاعدة البيانات

- [ ] Run migrations
- [ ] Set up database backups
- [ ] Configure connection pooling
- [ ] Set up read replicas (if needed)
- [ ] Enable slow query logging

---

## Monitoring | المراقبة

### Health Check Endpoint | نقطة فحص الصحة

```rust
router.get("/health", |_req| {
    Ok(Response::ok().json(&serde_json::json!({
        "status": "healthy",
        "version": noor::VERSION,
        "uptime": uptime_seconds,
    }))?)
});
```

### Metrics Endpoint | نقطة المقاييس

```rust
router.get("/metrics", |_req| {
    Ok(Response::ok()
        .header("content-type", "text/plain")
        .text(metrics_registry.export_prometheus()))
});
```

### Log Aggregation | تجميع السجلات

Configure structured JSON logging:

```toml
[log]
level = "info"
json = true
file = "storage/logs/app.log"
```

Use with:
- **ELK Stack** (Elasticsearch, Logstash, Kibana)
- **Grafana Loki**
- **Datadog**

---

## Scaling | التوسع

### Horizontal Scaling | التوسع الأفقي

```bash
# Run multiple instances
docker-compose up -d --scale noor=3

# With a load balancer
# Nginx will distribute traffic across instances
```

### Load Balancer Config | إعدادات موازن الحمل

```nginx
upstream noor_backend {
    server noor1:8080;
    server noor2:8080;
    server noor3:8080;
    keepalive 32;
}

server {
    location / {
        proxy_pass http://noor_backend;
    }
}
```

### Database Scaling | توسع قاعدة البيانات

- **Read Replicas** - For read-heavy workloads
- **Sharding** - For very large datasets
- **Connection Pooling** - PgBouncer for PostgreSQL

### Cache Scaling | توسع التخزين المؤقت

- **Redis Cluster** - For distributed caching
- **Redis Sentinel** - For high availability

---

## Backup Strategy | استراتيجية النسخ الاحتياطي

### Automated Backups | نسخ احتياطية تلقائية

```rust
use noor::core::scheduler::Scheduler;
use noor::core::backup::{BackupManager, BackupType};

let scheduler = Scheduler::new();
let backup_manager = Arc::new(BackupManager::new(BackupConfig::default())?);

scheduler.daily_at("daily_backup", 2, 0, move || {
    backup_manager.create_backup(BackupType::Database)?;
    Ok(())
});
```

### Backup Retention | الاحتفاظ بالنسخ

```rust
let config = BackupConfig {
    max_backups: 30,  // Keep 30 days of backups
    ..Default::default()
};
```

---

## Troubleshooting | استكشاف الأخطاء

### Common Issues | مشاكل شائعة

1. **Port already in use**
   ```bash
   lsof -i :8080
   kill -9 <PID>
   ```

2. **Permission denied**
   ```bash
   sudo chown -R noor:noor /var/lib/noor
   ```

3. **Database connection failed**
   - Check DATABASE_URL
   - Verify database is running
   - Check firewall rules

4. **Out of memory**
   - Use weak-server build profile
   - Increase swap space
   - Upgrade server RAM

### Log Locations | مواقع السجلات

- Application: `/var/lib/noor/storage/logs/app.log`
- Systemd: `journalctl -u noor -f`
- Nginx: `/var/log/nginx/error.log`

---

## Conclusion | خاتمة

Noor Framework is designed for easy deployment across various environments. Always test your deployment in a staging environment before production.

إطار عمل نور مصمم للنشر السهل في بيئات مختلفة. اختبر دائماً النشر في بيئة staging قبل الإنتاج.
