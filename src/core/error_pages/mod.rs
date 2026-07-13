// ============================================================
// Error Pages - صفحات الأخطاء
// ============================================================
// Beautiful, user-friendly HTML error pages for common HTTP errors.
// صفحات أخطاء HTML جميلة وصديقة للمستخدم.
// ============================================================

use crate::core::http::{Response, StatusCode};

/// Generate a beautiful HTML error page
pub fn render_error(status: StatusCode, message: Option<&str>) -> Response {
    let html = match status.0 {
        400 => bad_request(message),
        401 => unauthorized(message),
        403 => forbidden(message),
        404 => not_found(message),
        405 => method_not_allowed(message),
        418 => im_a_teapot(message),
        422 => unprocessable_entity(message),
        429 => too_many_requests(message),
        500 => internal_server_error(message),
        502 => bad_gateway(message),
        503 => service_unavailable(message),
        504 => gateway_timeout(message),
        _ => generic_error(status, message),
    };
    
    Response::new(status).html(html)
}

/// Get error icon SVG
fn error_icon(status: u16) -> &'static str {
    match status {
        404 => "🔍",
        401 => "🔐",
        403 => "⛔",
        500 => "💥",
        503 => "🚧",
        429 => "⏱️",
        418 => "🫖",
        _ => "⚠️",
    }
}

/// Base HTML template
fn base_template(status: u16, title: &str, icon: &str, message: &str, suggestion: Option<&str>) -> String {
    let suggestion_html = suggestion.map(|s| format!(
        r#"<div class="suggestion">
            <p>💡 <strong>اقتراح:</strong> {}</p>
        </div>"#,
        s
    )).unwrap_or_default();
    
    format!(
        r##"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Noor Framework</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Tahoma, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .error-container {{
            background: white;
            border-radius: 20px;
            padding: 60px 40px;
            text-align: center;
            max-width: 600px;
            width: 100%;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            animation: slideUp 0.5s ease-out;
        }}
        @keyframes slideUp {{
            from {{ opacity: 0; transform: translateY(30px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        .error-code {{
            font-size: 120px;
            font-weight: 900;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            line-height: 1;
            margin-bottom: 20px;
        }}
        .error-icon {{
            font-size: 80px;
            margin-bottom: 20px;
            display: block;
        }}
        .error-title {{
            font-size: 28px;
            color: #2c3e50;
            margin-bottom: 15px;
            font-weight: 700;
        }}
        .error-message {{
            font-size: 16px;
            color: #7f8c8d;
            line-height: 1.6;
            margin-bottom: 30px;
        }}
        .suggestion {{
            background: #f8f9fa;
            border-left: 4px solid #3498db;
            padding: 15px 20px;
            border-radius: 8px;
            margin-bottom: 30px;
            text-align: right;
            color: #2c3e50;
        }}
        .actions {{
            display: flex;
            gap: 15px;
            justify-content: center;
            flex-wrap: wrap;
        }}
        .btn {{
            display: inline-block;
            padding: 14px 32px;
            border-radius: 50px;
            text-decoration: none;
            font-weight: 600;
            font-size: 15px;
            transition: all 0.3s;
            cursor: pointer;
            border: none;
        }}
        .btn-primary {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        .btn-primary:hover {{
            transform: translateY(-2px);
            box-shadow: 0 10px 20px rgba(102, 126, 234, 0.4);
        }}
        .btn-secondary {{
            background: #ecf0f1;
            color: #2c3e50;
        }}
        .btn-secondary:hover {{
            background: #d5dbdb;
        }}
        .footer {{
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #ecf0f1;
            color: #bdc3c7;
            font-size: 13px;
        }}
        .footer a {{
            color: #3498db;
            text-decoration: none;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <span class="error-icon">{}</span>
        <div class="error-code">{}</div>
        <h1 class="error-title">{}</h1>
        <p class="error-message">{}</p>
        {}
        <div class="actions">
            <a href="/" class="btn btn-primary">🏠 العودة للرئيسية</a>
            <a href="javascript:history.back()" class="btn btn-secondary">← العودة للسابق</a>
        </div>
        <div class="footer">
            <p>Powered by <a href="#">Noor Framework</a> v{}</p>
        </div>
    </div>
</body>
</html>"##,
        title,
        icon,
        status,
        title,
        message,
        suggestion_html,
        crate::VERSION
    )
}

pub fn bad_request(message: Option<&str>) -> String {
    let msg = message.unwrap_or("الطلب غير صحيح. يرجى التحقق من البيانات المرسلة.");
    base_template(400, "طلب غير صحيح", error_icon(400), msg, Some("تحقق من صحة البيانات وأعد المحاولة."))
}

pub fn unauthorized(message: Option<&str>) -> String {
    let msg = message.unwrap_or("يجب تسجيل الدخول للوصول إلى هذه الصفحة.");
    base_template(401, "غير مصرح", error_icon(401), msg, Some("سجل دخولك للمتابعة."))
}

pub fn forbidden(message: Option<&str>) -> String {
    let msg = message.unwrap_or("ليس لديك صلاحية للوصول إلى هذه الصفحة.");
    base_template(403, "ممنوع الوصول", error_icon(403), msg, Some("تواصل مع المسؤول إذا كنت تعتقد أن هذا خطأ."))
}

pub fn not_found(message: Option<&str>) -> String {
    let msg = message.unwrap_or("عذراً، الصفحة التي تبحث عنها غير موجودة أو تم نقلها.");
    base_template(404, "الصفحة غير موجودة", error_icon(404), msg, Some("تحقق من الرابط أو ابحث عما تريد من الرئيسية."))
}

pub fn method_not_allowed(message: Option<&str>) -> String {
    let msg = message.unwrap_or("طريقة HTTP المستخدمة غير مسموحة لهذا المسار.");
    base_template(405, "الطريقة غير مسموحة", error_icon(405), msg, None)
}

pub fn unprocessable_entity(message: Option<&str>) -> String {
    let msg = message.unwrap_or("البيانات المرسلة غير صالحة. يرجى تصحيح الأخطاء والمحاولة مرة أخرى.");
    base_template(422, "بيانات غير صالحة", error_icon(422), msg, Some("راجع الأخطاء أدناه وصححها."))
}

pub fn too_many_requests(message: Option<&str>) -> String {
    let msg = message.unwrap_or("لقد قمت بإرسال عدد كبير من الطلبات. يرجى المحاولة مرة أخرى لاحقاً.");
    base_template(429, "طلبات كثيرة جداً", error_icon(429), msg, Some("انتظر دقيقة وأعد المحاولة."))
}

pub fn im_a_teapot(message: Option<&str>) -> String {
    let msg = message.unwrap_or("أنا إبريق شاي. لا أستطيع تحضير القهوة! ☕");
    base_template(418, "أنا إبريق شاي", error_icon(418), msg, Some("جرب تحضير الشاي بدلاً من ذلك! 😄"))
}

pub fn internal_server_error(message: Option<&str>) -> String {
    let msg = message.unwrap_or("حدث خطأ داخلي في الخادم. نحن نعمل على إصلاحه.");
    base_template(500, "خطأ في الخادم", error_icon(500), msg, Some("تم إبلاغ فريق الدعم تلقائياً. حاول مرة أخرى لاحقاً."))
}

pub fn bad_gateway(message: Option<&str>) -> String {
    let msg = message.unwrap_or("استجابة غير صحيحة من الخادم الخارجي.");
    base_template(502, "بوابة خاطئة", error_icon(502), msg, None)
}

pub fn service_unavailable(message: Option<&str>) -> String {
    let msg = message.unwrap_or("الخدمة غير متاحة حالياً بسبب صيانة أو سعة كاملة.");
    base_template(503, "الخدمة غير متاحة", error_icon(503), msg, Some("سنكون عائدين قريباً!"))
}

pub fn gateway_timeout(message: Option<&str>) -> String {
    let msg = message.unwrap_or("انتهت مهلة انتظار الاستجابة من الخادم.");
    base_template(504, "انتهت مهلة البوابة", error_icon(504), msg, None)
}

pub fn generic_error(status: StatusCode, message: Option<&str>) -> String {
    let msg = message.unwrap_or("حدث خطأ غير متوقع.");
    base_template(status.0, "خطأ", error_icon(status.0), msg, None)
}

/// Exception page for development (shows stack trace)
pub fn exception_page(error: &str, file: &str, line: u32) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Exception - Noor Framework (Development)</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'SF Mono', 'Monaco', 'Courier New', monospace;
            background: #1e1e1e;
            color: #d4d4d4;
            padding: 30px;
        }}
        .exception {{
            max-width: 900px;
            margin: 0 auto;
        }}
        .header {{
            background: #dc3545;
            color: white;
            padding: 20px;
            border-radius: 8px 8px 0 0;
            font-size: 18px;
            font-weight: bold;
        }}
        .content {{
            background: #2d2d2d;
            padding: 25px;
            border-radius: 0 0 8px 8px;
            border: 1px solid #3e3e42;
        }}
        .error-message {{
            font-size: 16px;
            color: #f44747;
            margin-bottom: 20px;
            padding: 15px;
            background: #1e1e1e;
            border-radius: 4px;
            border-left: 4px solid #f44747;
        }}
        .file-info {{
            color: #569cd6;
            margin-bottom: 20px;
        }}
        .file-info span {{ color: #ce9178; }}
        .help {{
            color: #608b4e;
            font-size: 14px;
            margin-top: 20px;
            padding: 15px;
            background: #1e1e1e;
            border-radius: 4px;
            border-left: 4px solid #608b4e;
        }}
    </style>
</head>
<body>
    <div class="exception">
        <div class="header">💥 Exception Thrown</div>
        <div class="content">
            <div class="error-message">{}</div>
            <div class="file-info">
                📁 File: <span>{}</span><br>
                📄 Line: <span>{}</span>
            </div>
            <div class="help">
                💡 This is a development error page. In production, users will see a friendly error page instead.
            </div>
        </div>
    </div>
</body>
</html>"#,
        error, file, line
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_render_not_found() {
        let response = render_error(StatusCode::NOT_FOUND, None);
        let html = String::from_utf8_lossy(&response.body);
        
        assert!(html.contains("404"));
        assert!(html.contains("الصفحة غير موجودة"));
    }
    
    #[test]
    fn test_render_internal_error() {
        let response = render_error(StatusCode::INTERNAL_SERVER_ERROR, Some("Custom error"));
        let html = String::from_utf8_lossy(&response.body);
        
        assert!(html.contains("500"));
        assert!(html.contains("Custom error"));
    }
    
    #[test]
    fn test_exception_page() {
        let html = exception_page("Something went wrong", "src/main.rs", 42);
        
        assert!(html.contains("Exception Thrown"));
        assert!(html.contains("src/main.rs"));
        assert!(html.contains("42"));
    }
}
