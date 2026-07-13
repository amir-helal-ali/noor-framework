// ============================================================
// Zig Performance Module - Main Entry
// وحدة الأداء بلغة Zig
// ============================================================
// This module exposes high-performance functions written in Zig
// that are called from Rust via FFI. These handle performance-
// critical paths like HTTP parsing and crypto operations.
//
// تصدر هذه الوحدة دوال عالية الأداء مكتوبة بلغة Zig.
// ============================================================

const std = @import("std");

// ============================================================
// HTTP Parser - Very fast HTTP/1.1 request parser
// محلل HTTP سريع جداً
// ============================================================

pub const HttpMethod = enum {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    UNKNOWN,
};

pub const HttpRequest = struct {
    method: HttpMethod,
    path: []const u8,
    query: []const u8,
    version: []const u8,
    headers: std.StringHashMap([]const u8),
    body: []const u8,
};

/// Parse an HTTP request from raw bytes
/// تحليل طلب HTTP من البايتات الخام
pub fn parse_http_request(data: []const u8) ?HttpRequest {
    // Find the end of headers
    const header_end = std.mem.indexOf(u8, data, "\r\n\r\n") orelse return null;
    const headers_section = data[0..header_end];
    const body = if (data.len > header_end + 4) data[header_end + 4..] else "";
    
    var lines = std.mem.splitSequence(u8, headers_section, "\r\n");
    const request_line = lines.next() orelse return null;
    
    // Parse request line: METHOD PATH HTTP/1.1
    var parts = std.mem.splitScalar(u8, request_line, ' ');
    const method_str = parts.next() orelse return null;
    const uri = parts.next() orelse return null;
    const version = parts.next() orelse return null;
    
    const method = parse_method(method_str);
    
    // Split path and query
    const path_end = std.mem.indexOfScalar(u8, uri, '?');
    const path = if (path_end) |end| uri[0..end] else uri;
    const query = if (path_end) |end| uri[end + 1..] else "";
    
    return HttpRequest{
        .method = method,
        .path = path,
        .query = query,
        .version = version,
        .headers = undefined, // Would be populated in full implementation
        .body = body,
    };
}

fn parse_method(s: []const u8) HttpMethod {
    if (std.mem.eql(u8, s, "GET")) return .GET;
    if (std.mem.eql(u8, s, "POST")) return .POST;
    if (std.mem.eql(u8, s, "PUT")) return .PUT;
    if (std.mem.eql(u8, s, "PATCH")) return .PATCH;
    if (std.mem.eql(u8, s, "DELETE")) return .DELETE;
    if (std.mem.eql(u8, s, "HEAD")) return .HEAD;
    if (std.mem.eql(u8, s, "OPTIONS")) return .OPTIONS;
    if (std.mem.eql(u8, s, "CONNECT")) return .CONNECT;
    if (std.mem.eql(u8, s, "TRACE")) return .TRACE;
    return .UNKNOWN;
}

// ============================================================
// Buffer Pool - Zero-allocation buffer recycling
// تجمع البافرات - إعادة استخدام بدون تخصيص
// ============================================================

pub const BufferPool = struct {
    buffers: std.ArrayList([]u8),
    buffer_size: usize,
    allocator: std.mem.Allocator,
    mutex: std.Thread.Mutex,
    
    pub fn init(allocator: std.mem.Allocator, buffer_size: usize) BufferPool {
        return .{
            .buffers = std.ArrayList([]u8).init(allocator),
            .buffer_size = buffer_size,
            .allocator = allocator,
            .mutex = .{},
        };
    }
    
    pub fn deinit(self: *BufferPool) void {
        for (self.buffers.items) |buf| {
            self.allocator.free(buf);
        }
        self.buffers.deinit();
    }
    
    /// Get a buffer from the pool (or allocate if empty)
    pub fn acquire(self: *BufferPool) ![]u8 {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        if (self.buffers.items.len > 0) {
            return self.buffers.pop();
        }
        
        return self.allocator.alloc(u8, self.buffer_size);
    }
    
    /// Return a buffer to the pool for reuse
    pub fn release(self: *BufferPool, buf: []u8) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        self.buffers.append(buf) catch {
            self.allocator.free(buf);
        };
    }
};

// ============================================================
// Fast CRC32 - For cache key hashing
/// CRC32 سريع - لتجزئة مفاتيح الكاش
// ============================================================

const crc32_table = blk: {
    @setEvalBranchQuota(2000);
    var table: [256]u32 = undefined;
    
    var i: u32 = 0;
    while (i < 256) : (i += 1) {
        var crc: u32 = i;
        var j: u32 = 0;
        while (j < 8) : (j += 1) {
            if (crc & 1 != 0) {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
        table[i] = crc;
    }
    
    break :blk table;
};

pub fn crc32(data: []const u8) u32 {
    var crc: u32 = 0xFFFFFFFF;
    
    for (data) |byte| {
        const index = (crc ^ byte) & 0xFF;
        crc = (crc >> 8) ^ crc32_table[index];
    }
    
    return crc ^ 0xFFFFFFFF;
}

// ============================================================
// Fast String Comparison (constant-time for security)
/// مقارنة نصوص بزمن ثابت للأمان
// ============================================================

pub fn constant_time_compare(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    
    var result: u8 = 0;
    for (a, b) |a_byte, b_byte| {
        result |= a_byte ^ b_byte;
    }
    
    return result == 0;
}

// ============================================================
// URL Decoder - Fast URL decoding
/// فاك تشفير URL سريع
// ============================================================

pub fn url_decode(allocator: std.mem.Allocator, input: []const u8) ![]u8 {
    var output = try allocator.alloc(u8, input.len);
    var output_len: usize = 0;
    
    var i: usize = 0;
    while (i < input.len) {
        if (input[i] == '%' and i + 2 < input.len) {
            const high = hex_to_nibble(input[i + 1]) orelse {
                output[output_len] = input[i];
                output_len += 1;
                i += 1;
                continue;
            };
            const low = hex_to_nibble(input[i + 2]) orelse {
                output[output_len] = input[i];
                output_len += 1;
                i += 1;
                continue;
            };
            
            output[output_len] = (high << 4) | low;
            output_len += 1;
            i += 3;
        } else if (input[i] == '+') {
            output[output_len] = ' ';
            output_len += 1;
            i += 1;
        } else {
            output[output_len] = input[i];
            output_len += 1;
            i += 1;
        }
    }
    
    return output[0..output_len];
}

fn hex_to_nibble(c: u8) ?u8 {
    return switch (c) {
        '0'...'9' => c - '0',
        'a'...'f' => c - 'a' + 10,
        'A'...'F' => c - 'A' + 10,
        else => null,
    };
}

// ============================================================
// Tests
// ============================================================

test "crc32" {
    const result = crc32("Hello, World!");
    try std.testing.expect(result != 0);
}

test "constant_time_compare" {
    try std.testing.expect(constant_time_compare("hello", "hello"));
    try std.testing.expect(!constant_time_compare("hello", "world"));
    try std.testing.expect(!constant_time_compare("hello", "hell"));
}

test "url_decode" {
    const allocator = std.testing.allocator;
    const decoded = try url_decode(allocator, "hello%20world");
    defer allocator.free(decoded);
    try std.testing.expectEqualStrings("hello world", decoded);
}

test "buffer_pool" {
    const allocator = std.testing.allocator;
    var pool = BufferPool.init(allocator, 1024);
    defer pool.deinit();
    
    const buf1 = try pool.acquire();
    pool.release(buf1);
    
    const buf2 = try pool.acquire();
    try std.testing.expectEqual(buf1.ptr, buf2.ptr);
    pool.release(buf2);
}
