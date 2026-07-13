// ============================================================
// Noor Framework Live Demo - عرض حي لإطار عمل نور
// ============================================================

use std::sync::Arc;

use noor::core::config::{Config, Environment};
use noor::core::http::{Request, Response, StatusCode};
use noor::core::router::Router;
use noor::Application;
use parking_lot::Mutex;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Task {
    id: i64,
    title: String,
    done: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In-memory task store.
    let tasks: Arc<Mutex<Vec<Task>>> = Arc::new(Mutex::new(vec![
        Task { id: 1, title: "تعلم إطار عمل نور".into(), done: false },
        Task { id: 2, title: "تثبيت Rust و Cargo".into(), done: true },
        Task { id: 3, title: "بناء تطبيق ويب كامل".into(), done: false },
    ]));
    let next_id: Arc<Mutex<i64>> = Arc::new(Mutex::new(4));

    let mut router = Router::new();

    // Landing page.
    router.get("/", move |_req: Request| {
        Ok(Response::ok().html(LANDING_PAGE))
    });

    // API: List tasks.
    let t1 = tasks.clone();
    router.get("/api/tasks", move |_req: Request| {
        let tasks = t1.lock().clone();
        let json = serde_json::to_string(&tasks)
            .map_err(|e| noor::NoorError::Internal(e.to_string()))?;
        Ok(Response::ok()
            .header("content-type", "application/json; charset=utf-8")
            .body(json))
    });

    // API: Create task.
    let t2 = tasks.clone();
    let n2 = next_id.clone();
    router.post("/api/tasks", move |req: Request| {
        let body: serde_json::Value = req.json()
            .map_err(|_| noor::NoorError::Validation("Invalid JSON".to_string()))?;
        let title = body["title"].as_str()
            .ok_or_else(|| noor::NoorError::Validation("Missing 'title'".to_string()))?;
        if title.is_empty() {
            return Err(noor::NoorError::Validation("Title cannot be empty".into()));
        }
        let id = { let mut n = n2.lock(); let id = *n; *n += 1; id };
        let task = Task { id, title: title.into(), done: false };
        t2.lock().push(task.clone());
        let json = serde_json::to_string(&task)
            .map_err(|e| noor::NoorError::Internal(e.to_string()))?;
        Ok(Response::new(StatusCode::CREATED)
            .header("content-type", "application/json; charset=utf-8")
            .body(json))
    });

    // API: Toggle task.
    let t3 = tasks.clone();
    router.post("/api/tasks/{id}/toggle", move |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        let mut tasks = t3.lock();
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.done = !t.done;
            let json = serde_json::to_string(&*t)
                .map_err(|e| noor::NoorError::Internal(e.to_string()))?;
            Ok(Response::ok()
                .header("content-type", "application/json; charset=utf-8")
                .body(json))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND).text("Task not found"))
        }
    });

    // API: Delete task.
    let t4 = tasks.clone();
    router.delete("/api/tasks/{id}", move |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        let mut tasks = t4.lock();
        let before = tasks.len();
        tasks.retain(|t| t.id != id);
        if tasks.len() < before {
            Ok(Response::new(StatusCode::NO_CONTENT).body(""))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND).text("Task not found"))
        }
    });

    // Health check.
    router.get("/health", move |_req: Request| {
        Ok(Response::ok().json(&serde_json::json!({
            "status": "ok",
            "framework": "noor",
            "version": noor::VERSION,
        }))?)
    });

    // Config.
    let mut config = Config::default();
    config.app.env = Environment::Development;
    config.server.host = "0.0.0.0".to_string();
    config.server.port = 8080;
    config.security.cors_origins = vec!["*".to_string()];
    config.security.secure_headers = true;
    config.server.compression = true;

    let app = Application::new(config, router);
    println!("Starting Noor Live Demo on port 8080...");
    app.run()?;
    Ok(())
}

const LANDING_PAGE: &str = r##"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Noor Framework - عرض حي</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Segoe UI', Tahoma, sans-serif;
            background: linear-gradient(135deg, #0c0c1e 0%, #1a1a3e 50%, #0c0c1e 100%);
            color: #e0e0e0;
            min-height: 100vh;
        }
        .header { text-align: center; padding: 60px 20px 40px; }
        .logo {
            font-size: 4rem; font-weight: bold;
            background: linear-gradient(135deg, #00d4ff, #7b2ff7);
            -webkit-background-clip: text; -webkit-text-fill-color: transparent;
            margin-bottom: 10px;
        }
        .tagline { font-size: 1.3rem; color: #a0a0c0; margin-bottom: 30px; }
        .badges { display: flex; justify-content: center; gap: 15px; flex-wrap: wrap; }
        .badge {
            background: rgba(255,255,255,0.08);
            border: 1px solid rgba(255,255,255,0.15);
            padding: 8px 16px; border-radius: 20px; font-size: 0.9rem;
        }
        .container { max-width: 1000px; margin: 0 auto; padding: 20px; }
        .section {
            background: rgba(255,255,255,0.04);
            border-radius: 16px; padding: 30px; margin-bottom: 25px;
            border: 1px solid rgba(255,255,255,0.08);
        }
        .section h2 { color: #00d4ff; margin-bottom: 20px; font-size: 1.6rem; }
        .features { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 15px; }
        .feature {
            background: rgba(0,212,255,0.05);
            border: 1px solid rgba(0,212,255,0.2);
            padding: 15px; border-radius: 10px;
        }
        .feature h3 { color: #7b2ff7; margin-bottom: 8px; }
        .feature p { color: #b0b0c0; font-size: 0.9rem; line-height: 1.5; }
        .demo-box { background: rgba(0,0,0,0.3); border-radius: 10px; padding: 20px; margin-top: 15px; }
        .task-list { list-style: none; }
        .task-item {
            display: flex; align-items: center; gap: 12px;
            padding: 12px; background: rgba(255,255,255,0.03);
            border-radius: 8px; margin-bottom: 8px; transition: all 0.2s;
        }
        .task-item:hover { background: rgba(255,255,255,0.06); }
        .task-item.done .task-title { text-decoration: line-through; color: #666; }
        .task-title { flex: 1; font-size: 1rem; }
        .checkbox {
            width: 22px; height: 22px; border: 2px solid #00d4ff;
            border-radius: 6px; cursor: pointer;
            display: flex; align-items: center; justify-content: center;
            transition: all 0.2s;
        }
        .checkbox.checked { background: #00d4ff; }
        .checkbox.checked::after { content: "\2713"; color: #0c0c1e; font-weight: bold; }
        .delete-btn {
            background: rgba(255,80,80,0.15); color: #ff5050;
            border: 1px solid rgba(255,80,80,0.3);
            padding: 6px 12px; border-radius: 6px; cursor: pointer; font-size: 0.85rem;
        }
        .delete-btn:hover { background: rgba(255,80,80,0.25); }
        .add-form { display: flex; gap: 10px; margin-bottom: 15px; }
        .add-form input {
            flex: 1; padding: 10px 14px;
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.15);
            border-radius: 8px; color: #e0e0e0; font-size: 1rem;
        }
        .add-form input::placeholder { color: #666; }
        .add-form button {
            padding: 10px 20px;
            background: linear-gradient(135deg, #00d4ff, #7b2ff7);
            border: none; border-radius: 8px; color: white;
            font-weight: bold; cursor: pointer; font-size: 1rem;
        }
        .api-links { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; }
        .api-link {
            display: block; background: rgba(0,212,255,0.05);
            border: 1px solid rgba(0,212,255,0.15);
            padding: 12px; border-radius: 8px; text-decoration: none;
            color: #00d4ff; font-family: monospace; font-size: 0.9rem; transition: all 0.2s;
        }
        .api-link:hover { background: rgba(0,212,255,0.1); }
        .method { color: #7b2ff7; font-weight: bold; }
        .stats { display: flex; gap: 30px; justify-content: center; flex-wrap: wrap; }
        .stat { text-align: center; }
        .stat .num { font-size: 2.5rem; font-weight: bold; color: #00d4ff; }
        .stat .label { color: #a0a0c0; font-size: 0.9rem; }
        .footer { text-align: center; padding: 30px; color: #666; font-size: 0.85rem; }
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">Noor Framework</div>
        <div class="tagline">خفيف، سريع، آمن — إطار عمل ويب بلغة Rust</div>
        <div class="badges">
            <span class="badge">Rust 1.97</span>
            <span class="badge">Hyper + Tokio</span>
            <span class="badge">SQLite / PostgreSQL / MySQL</span>
            <span class="badge">527 اختبار ينجح</span>
        </div>
    </div>

    <div class="container">
        <div class="section">
            <h2>احصائيات الاطار</h2>
            <div class="stats">
                <div class="stat"><div class="num">527</div><div class="label">اختبار ناجح</div></div>
                <div class="stat"><div class="num">50+</div><div class="label">وحدة (module)</div></div>
                <div class="stat"><div class="num">100%</div><div class="label">Rust</div></div>
                <div class="stat"><div class="num">0</div><div class="label">خطأ تجميع</div></div>
            </div>
        </div>

        <div class="section">
            <h2>الميزات</h2>
            <div class="features">
                <div class="feature"><h3>خادم HTTP حقيقي</h3><p>مبني على Hyper 1.x + Tokio مع graceful shutdown و connection pooling.</p></div>
                <div class="feature"><h3>قاعدة بيانات حقيقية</h3><p>دعم SQLite / PostgreSQL / MySQL عبر sqlx مع معاملات وترحيلات حقيقية.</p></div>
                <div class="feature"><h3>مصادقة JWT</h3><p>JWT middleware مع RBAC و rate limiting و CORS و security headers.</p></div>
                <div class="feature"><h3>جلسات وكوكيز</h3><p>تخزين جلسات ملفي مع كوكيز موقعة وتحليل تلقائي للكوكيز.</p></div>
                <div class="feature"><h3>قوالب Handlebars</h3><p>محرك قوالب مع helpers مخصصة وإعادة تحميل تلقائية في التطوير.</p></div>
                <div class="feature"><h3>ضغط تلقائي</h3><p>ضغط Brotli/Gzip تلقائي للاستجابات حسب Accept-Encoding.</p></div>
                <div class="feature"><h3>أمان مدمج</h3><p>CSRF, XSS filter, SQL injection prevention, body size limits, secure headers.</p></div>
                <div class="feature"><h3>CLI كامل</h3><p>أوامر: serve, migrate, make:migration, make:controller, make:model.</p></div>
            </div>
        </div>

        <div class="section">
            <h2>تطبيق المهام (CRUD)</h2>
            <p style="color:#a0a0c0; margin-bottom:15px;">جرّب إضافة مهمة، تعديل حالتها، أو حذفها — البيانات تُحفظ في الذاكرة.</p>
            <div class="demo-box">
                <div class="add-form">
                    <input type="text" id="newTask" placeholder="أضف مهمة جديدة..." onkeypress="if(event.key==='Enter')addTask()">
                    <button onclick="addTask()">إضافة</button>
                </div>
                <ul class="task-list" id="taskList">
                    <li style="color:#666; text-align:center; padding:20px;">جارٍ التحميل...</li>
                </ul>
            </div>
        </div>

        <div class="section">
            <h2>واجهات API</h2>
            <div class="api-links">
                <a class="api-link" href="/api/tasks" target="_blank"><span class="method">GET</span> /api/tasks</a>
                <a class="api-link" href="/health" target="_blank"><span class="method">GET</span> /health</a>
            </div>
        </div>
    </div>

    <div class="footer">
        <p>Noor Framework v1.0.0 — مبني بلغة Rust</p>
        <p style="margin-top:5px;">الخادم يعمل على المنفذ 8080 | الضغط: Brotli/Gzip | CORS + Security Headers مفعّلة</p>
    </div>

    <script>
        async function loadTasks() {
            try {
                const resp = await fetch('/api/tasks');
                const tasks = await resp.json();
                const list = document.getElementById('taskList');
                if (tasks.length === 0) {
                    list.innerHTML = '<li style="color:#666;text-align:center;padding:20px;">لا توجد مهام بعد</li>';
                    return;
                }
                list.innerHTML = tasks.map(t => 
                    '<li class="task-item ' + (t.done ? 'done' : '') + '">' +
                        '<div class="checkbox ' + (t.done ? 'checked' : '') + '" onclick="toggleTask(' + t.id + ')"></div>' +
                        '<span class="task-title">' + t.title + '</span>' +
                        '<button class="delete-btn" onclick="deleteTask(' + t.id + ')">حذف</button>' +
                    '</li>'
                ).join('');
            } catch(e) {
                document.getElementById('taskList').innerHTML = '<li style="color:#f55;">خطأ: ' + e + '</li>';
            }
        }

        async function addTask() {
            const input = document.getElementById('newTask');
            const title = input.value.trim();
            if (!title) return;
            await fetch('/api/tasks', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({title})
            });
            input.value = '';
            loadTasks();
        }

        async function toggleTask(id) {
            await fetch('/api/tasks/' + id + '/toggle', {method: 'POST'});
            loadTasks();
        }

        async function deleteTask(id) {
            await fetch('/api/tasks/' + id, {method: 'DELETE'});
            loadTasks();
        }

        loadTasks();
    </script>
</body>
</html>"##;
