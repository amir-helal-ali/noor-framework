/* ============================================================
   Noor Framework - Default JavaScript
   جافاسكريبت الافتراضي لإطار عمل نور
   ============================================================ */

(function() {
    'use strict';
    
    // Noor namespace
    window.Noor = {
        version: '1.0.0',
        
        // AJAX helper
        ajax: function(method, url, data, callback) {
            const xhr = new XMLHttpRequest();
            xhr.open(method, url, true);
            xhr.setRequestHeader('X-Requested-With', 'XMLHttpRequest');
            
            if (data && !(data instanceof FormData)) {
                xhr.setRequestHeader('Content-Type', 'application/json');
            }
            
            xhr.onload = function() {
                let response = xhr.responseText;
                try {
                    response = JSON.parse(response);
                } catch(e) {
                    // Not JSON, use raw text
                }
                
                if (callback) {
                    callback(response, xhr.status, xhr);
                }
            };
            
            xhr.onerror = function() {
                if (callback) {
                    callback(null, 0, xhr);
                }
            };
            
            xhr.send(data instanceof FormData ? data : (data ? JSON.stringify(data) : null));
        },
        
        // Get CSRF token from meta tag
        getCsrfToken: function() {
            const meta = document.querySelector('meta[name="csrf-token"]');
            return meta ? meta.getAttribute('content') : null;
        },
        
        // Form validation helper
        validateForm: function(form) {
            const errors = [];
            const inputs = form.querySelectorAll('input[required], textarea[required], select[required]');
            
            inputs.forEach(function(input) {
                if (!input.value.trim()) {
                    errors.push({
                        field: input.name,
                        message: input.name + ' is required'
                    });
                }
            });
            
            // Email validation
            const emails = form.querySelectorAll('input[type="email"]');
            emails.forEach(function(input) {
                if (input.value && !input.value.match(/^[^\s@]+@[^\s@]+\.[^\s@]+$/)) {
                    errors.push({
                        field: input.name,
                        message: 'Invalid email format'
                    });
                }
            });
            
            return errors;
        },
        
        // Show alert
        showAlert: function(message, type) {
            type = type || 'info';
            const alert = document.createElement('div');
            alert.className = 'alert alert-' + type;
            alert.textContent = message;
            
            const container = document.querySelector('.container') || document.body;
            container.insertBefore(alert, container.firstChild);
            
            setTimeout(function() {
                alert.remove();
            }, 5000);
        },
        
        // Confirm dialog
        confirm: function(message) {
            return window.confirm(message);
        },
        
        // Format date
        formatDate: function(dateString) {
            const date = new Date(dateString);
            return date.toLocaleDateString('ar', {
                year: 'numeric',
                month: 'long',
                day: 'numeric'
            });
        }
    };
    
    // Auto-setup on DOM ready
    document.addEventListener('DOMContentLoaded', function() {
        // Add CSRF token to all forms
        const token = Noor.getCsrfToken();
        if (token) {
            const forms = document.querySelectorAll('form[method="POST"], form[method="post"]');
            forms.forEach(function(form) {
                const input = document.createElement('input');
                input.type = 'hidden';
                input.name = '_token';
                input.value = token;
                form.appendChild(input);
            });
        }
        
        // Form validation
        const forms = document.querySelectorAll('form[data-validate]');
        forms.forEach(function(form) {
            form.addEventListener('submit', function(e) {
                const errors = Noor.validateForm(form);
                if (errors.length > 0) {
                    e.preventDefault();
                    Noor.showAlert(errors[0].message, 'danger');
                }
            });
        });
        
        // Confirm dialogs
        const confirmElements = document.querySelectorAll('[data-confirm]');
        confirmElements.forEach(function(el) {
            el.addEventListener('click', function(e) {
                if (!Noor.confirm(el.getAttribute('data-confirm'))) {
                    e.preventDefault();
                }
            });
        });
    });
})();
