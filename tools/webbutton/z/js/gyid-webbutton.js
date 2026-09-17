/**
 * GYID WebButton - 主入口文件
 * 基于 GEOYUAN 网站弹窗设计规范
 * 
 * 设计参考: http://daixie.uno/2026fakao/
 */

(function(global) {
  'use strict';

  // ================================
  // 图标 SVG
  // ================================
  const Icons = {
    location: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="10" r="3"></circle>
      <path d="M12 2a8 8 0 0 0-8 8c0 5.4 7 11.5 7.3 11.8a1 1 0 0 0 1.4 0C13 21.5 20 15.4 20 10a8 8 0 0 0-8-8z"></path>
    </svg>`,
    close: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M18 6L6 18M6 6l12 12"></path>
    </svg>`,
    fingerprint: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M2 12C2 6.5 6.5 2 12 2a10 10 0 0 1 8 4"></path>
      <path d="M5 19.5C5.5 18 6 15 6 12c0-.7.12-1.37.34-2"></path>
      <path d="M17.29 21.02c.12-.6.43-2.3.5-3.02"></path>
      <path d="M12 10a2 2 0 0 0-2 2c0 1.02-.1 2.51-.26 4"></path>
      <path d="M8.65 22c.21-.66.45-1.32.57-2"></path>
      <path d="M14 13.12c0 2.38 0 6.38-1 8.88"></path>
      <path d="M2 16h.01"></path>
      <path d="M21.8 16c.2-2 .131-5.354 0-6"></path>
      <path d="M9 6.8a6 6 0 0 1 9 5.2c0 .47 0 1.17-.02 2"></path>
    </svg>`,
    geo: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="10"></circle>
      <line x1="2" y1="12" x2="22" y2="12"></line>
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
    </svg>`,
    avatar: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
      <circle cx="12" cy="7" r="4"></circle>
    </svg>`,
    chain: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
    </svg>`,
    check: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <polyline points="20 6 9 17 4 12"></polyline>
    </svg>`,
    upload: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
      <polyline points="17 8 12 3 7 8"></polyline>
      <line x1="12" y1="3" x2="12" y2="15"></line>
    </svg>`,
    download: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
      <polyline points="7 10 12 15 17 10"></polyline>
      <line x1="12" y1="15" x2="12" y2="3"></line>
    </svg>`,
    copy: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
    </svg>`,
    polygon: `<svg viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2L3 7v10l9 5 9-5V7l-9-5zm0 2.18l7 3.89v7.86l-7 3.89-7-3.89V8.07l7-3.89z"/>
    </svg>`,
    aptos: `<svg viewBox="0 0 24 24" fill="currentColor">
      <circle cx="12" cy="12" r="10"/>
      <path d="M12 6v12M6 12h12" stroke="white" stroke-width="2"/>
    </svg>`,
    local: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
      <line x1="9" y1="3" x2="9" y2="21"></line>
    </svg>`,
    generate: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
    </svg>`
  };

  // ================================
  // 指纹采集模块
  // ================================
  class FingerprintCollector {
    constructor() {
      this.data = {};
    }

    async collect() {
      this.data = {
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        screen: `${screen.width}x${screen.height}`,
        colorDepth: screen.colorDepth,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        language: navigator.language,
        languages: navigator.languages?.join(',') || navigator.language,
        hardwareConcurrency: navigator.hardwareConcurrency,
        deviceMemory: navigator.deviceMemory,
        canvasHash: await this.getCanvasHash(),
        webglHash: this.getWebGLHash(),
        webglParams: this.getWebGLParams()
      };
      return this.data;
    }

    async getCanvasHash() {
      try {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        canvas.width = 200;
        canvas.height = 50;
        
        ctx.textBaseline = 'top';
        ctx.font = "14px 'Arial'";
        ctx.fillStyle = '#f60';
        ctx.fillRect(125, 1, 62, 20);
        ctx.fillStyle = '#069';
        ctx.fillText('GyID WebButton', 2, 15);
        ctx.fillStyle = 'rgba(102, 204, 0, 0.7)';
        ctx.fillText('GyID WebButton', 4, 17);
        
        const dataUrl = canvas.toDataURL();
        return await this.hashString(dataUrl);
      } catch (e) {
        return 'unavailable';
      }
    }

    getWebGLHash() {
      try {
        const canvas = document.createElement('canvas');
        const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
        if (!gl) return 'unavailable';
        
        const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
        const vendor = debugInfo ? gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) : '';
        const renderer = debugInfo ? gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) : '';
        return this.hashString(vendor + renderer);
      } catch (e) {
        return 'unavailable';
      }
    }

    getWebGLParams() {
      try {
        const canvas = document.createElement('canvas');
        const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
        if (!gl) return {};
        
        return {
          vendor: gl.getParameter(gl.VENDOR),
          renderer: gl.getParameter(gl.RENDERER),
          version: gl.getParameter(gl.VERSION),
          shadingLanguageVersion: gl.getParameter(gl.SHADING_LANGUAGE_VERSION)
        };
      } catch (e) {
        return {};
      }
    }

    async hashString(str) {
      const encoder = new TextEncoder();
      const data = encoder.encode(str);
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    }

    getSummary() {
      return {
        userAgent: { label: 'User Agent', value: this.truncate(this.data.userAgent, 30), collected: !!this.data.userAgent },
        screen: { label: '屏幕', value: this.data.screen, collected: !!this.data.screen },
        timezone: { label: '时区', value: this.data.timezone, collected: !!this.data.timezone },
        canvas: { label: 'Canvas指纹', value: this.truncate(this.data.canvasHash, 12) + '...', collected: this.data.canvasHash !== 'unavailable' }
      };
    }

    truncate(str, len) {
      return str ? (str.length > len ? str.substring(0, len) : str) : '';
    }
  }

  // ================================
  // 地理位置模块
  // ================================
  class GeoService {
    constructor() {
      this.precision = 'L1';
      this.location = null;
    }

    setPrecision(precision) {
      this.precision = precision;
    }

    async getLocation() {
      if (this.precision === 'L1' || this.precision === 'L2') {
        return await this.getIpLocation();
      }
      return await this.getGpsLocation();
    }

    async getGpsLocation() {
      return new Promise((resolve, reject) => {
        if (!navigator.geolocation) {
          resolve(this.getFallbackLocation());
          return;
        }

        navigator.geolocation.getCurrentPosition(
          async (position) => {
            const { latitude, longitude } = position.coords;
            const location = {
              latitude,
              longitude,
              accuracy: position.coords.accuracy,
              source: 'gps'
            };

            // 尝试获取城市信息
            try {
              const cityInfo = await this.reverseGeocode(latitude, longitude);
              location.city = cityInfo.city;
              location.country = cityInfo.country;
            } catch (e) {
              location.city = '未知';
              location.country = '未知';
            }

            this.location = location;
            resolve(location);
          },
          (error) => {
            console.warn('GPS定位失败，降级到IP定位:', error.message);
            this.getIpLocation().then(resolve);
          },
          {
            enableHighAccuracy: this.precision === 'L3',
            timeout: 10000,
            maximumAge: 300000
          }
        );
      });
    }

    async getIpLocation() {
      try {
        const response = await fetch('https://ip-api.com/json/?fields=status,country,city,lat,lon', {
          signal: AbortSignal.timeout(5000)
        });
        const data = await response.json();
        
        if (data.status === 'success') {
          this.location = {
            latitude: data.lat,
            longitude: data.lon,
            city: data.city,
            country: data.country,
            source: 'ip'
          };
          return this.location;
        }
      } catch (e) {
        console.warn('IP定位失败:', e.message);
      }
      
      return this.getFallbackLocation();
    }

    getFallbackLocation() {
      this.location = {
        latitude: 30.2741,
        longitude: 120.1551,
        city: '杭州市',
        country: '中国',
        source: 'fallback'
      };
      return this.location;
    }

    async reverseGeocode(lat, lon) {
      try {
        const response = await fetch(
          `https://nominatim.openstreetmap.org/reverse?format=json&lat=${lat}&lon=${lon}&accept-language=zh`,
          {
            headers: { 'User-Agent': 'GyID-WebButton/1.0' },
            signal: AbortSignal.timeout(5000)
          }
        );
        const data = await response.json();
        return {
          city: data.address?.city || data.address?.town || data.address?.village || '未知',
          country: data.address?.country || '未知'
        };
      } catch (e) {
        return { city: '未知', country: '未知' };
      }
    }

    getDisplayText() {
      if (!this.location) return '定位中...';
      if (this.location.source === 'gps') {
        return `${this.location.city}, ${this.location.country} (GPS)`;
      }
      return `${this.location.city}, ${this.location.country}`;
    }
  }

  // ================================
  // 头像采集模块
  // ================================
  class AvatarCollector {
    constructor() {
      this.imageData = null;
      this.preview = null;
    }

    setPreview(element) {
      this.preview = element;
    }

    async handleFile(file) {
      if (!file || !file.type.startsWith('image/')) {
        throw new Error('请选择图片文件');
      }

      if (file.size > 5 * 1024 * 1024) {
        throw new Error('图片大小不能超过5MB');
      }

      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (e) => {
          this.imageData = e.target.result;
          if (this.preview) {
            this.preview.innerHTML = `<img src="${this.imageData}" alt="Avatar">`;
          }
          resolve(this.imageData);
        };
        reader.onerror = () => reject(new Error('读取图片失败'));
        reader.readAsDataURL(file);
      });
    }

    async getHash() {
      if (!this.imageData) return null;
      
      const img = new Image();
      img.src = this.imageData;
      
      await new Promise((resolve, reject) => {
        img.onload = resolve;
        img.onerror = reject;
      });

      const canvas = document.createElement('canvas');
      const size = 128;
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(img, 0, 0, size, size);
      
      const dataUrl = canvas.toDataURL('image/png');
      const encoder = new TextEncoder();
      const data = encoder.encode(dataUrl);
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      return 'sha256:' + hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    }

    clear() {
      this.imageData = null;
      if (this.preview) {
        this.preview.innerHTML = Icons.avatar;
      }
    }
  }

  // ================================
  // GYID 生成器核心
  // ================================
  class GyIdGenerator {
    constructor() {
      this.fingerprint = null;
      this.geo = null;
      this.avatar = null;
      this.chain = 'local';
      this.timestamp = null;
    }

    setData(data) {
      this.fingerprint = data.fingerprint;
      this.geo = data.geo;
      this.avatar = data.avatar;
      this.chain = data.chain;
    }

    generate() {
      this.timestamp = Date.now();
      
      // 构建输入数据
      const input = {
        fp: this.fingerprint?.canvasHash?.substring(0, 16) || this.generateRandomHash(16),
        geo: this.geo ? `${this.geo.latitude.toFixed(6)},${this.geo.longitude.toFixed(6)}` : 'none',
        city: this.geo?.city || 'unknown',
        ts: this.timestamp,
        rnd: Math.random().toString(36).substring(2, 8),
        chain: this.chain
      };

      // 生成 GYID (简化版，实际应使用 WASM)
      const gyid = this.generateGyId(input);
      
      // 计算权重
      const weights = this.calculateWeights();

      return {
        gyid,
        input,
        weights,
        createdAt: new Date(this.timestamp).toISOString(),
        qrData: gyid
      };
    }

    generateGyId(input) {
      // 简化版 GYID 生成
      // 实际应该使用 Rust WASM 模块
      const prefix = 'GY';
      const version = '1';
      
      // 组合所有输入
      const combined = [
        input.fp,
        input.geo,
        input.ts.toString(),
        input.rnd,
        input.chain
      ].join('|');

      // 生成 hash
      const hash = this.simpleHash(combined);
      
      // Base58 编码
      const base58 = this.toBase58(hash);
      
      // 格式: GY1XXXXXXXXXXXXXXXXXXXXX
      return `${prefix}${version}${base58}`.substring(0, 24);
    }

    simpleHash(str) {
      let hash = 0;
      for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
      }
      return Math.abs(hash).toString(16).padStart(16, '0');
    }

    toBase58(hex) {
      const chars = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
      let num = BigInt('0x' + hex);
      let result = '';
      while (num > 0n) {
        result = chars[Number(num % 58n)] + result;
        num = num / 58n;
      }
      return result || '1';
    }

    generateRandomHash(len) {
      const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
      let result = '';
      for (let i = 0; i < len; i++) {
        result += chars.charAt(Math.floor(Math.random() * chars.length));
      }
      return result;
    }

    calculateWeights() {
      let total = 0;
      const weights = {
        fingerprint: this.fingerprint?.canvasHash ? 0.40 : 0,
        geo: this.geo ? 0.35 : 0,
        avatar: this.avatar ? 0.15 : 0,
        timestamp: 0.10
      };

      total = weights.fingerprint + weights.geo + weights.avatar + weights.timestamp;
      
      return {
        ...weights,
        total,
        isValid: total >= 0.6
      };
    }
  }

  // ================================
  // 二维码生成器
  // ================================
  class QRGenerator {
    constructor() {
      this.qr = null;
    }

    async generate(element, data, options = {}) {
      // 如果全局 QRCode 可用，使用它
      if (typeof QRCode !== 'undefined') {
        element.innerHTML = '';
        QRCode.toCanvas(data, {
          width: options.width || 104,
          margin: 1,
          color: {
            dark: '#0d1b3e',
            light: '#ffffff'
          },
          ...options
        }, (error, canvas) => {
          if (!error) {
            element.appendChild(canvas);
          }
        });
        return;
      }

      // 内联 QR 码生成（简化版）
      this.generateSimpleQR(element, data);
    }

    generateSimpleQR(element, data) {
      // 使用 SVG 生成简化 QR 码
      const size = 104;
      const moduleSize = 3;
      const modules = this.getQRMatrix(data);
      
      let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}">`;
      svg += `<rect width="${size}" height="${size}" fill="white"/>`;
      
      for (let y = 0; y < modules.length; y++) {
        for (let x = 0; x < modules[y].length; x++) {
          if (modules[y][x]) {
            svg += `<rect x="${x * moduleSize + 2}" y="${y * moduleSize + 2}" width="${moduleSize}" height="${moduleSize}" fill="#0d1b3e"/>`;
          }
        }
      }
      
      svg += '</svg>';
      element.innerHTML = svg;
    }

    getQRMatrix(data) {
      // 简化版 QR 矩阵生成
      // 实际应该使用完整的 QR 码算法
      const size = 21;
      const matrix = Array(size).fill(null).map(() => Array(size).fill(false));
      
      // 添加定位图案
      this.addFinderPattern(matrix, 0, 0);
      this.addFinderPattern(matrix, size - 7, 0);
      this.addFinderPattern(matrix, 0, size - 7);
      
      // 添加时序图案
      for (let i = 8; i < size - 8; i++) {
        matrix[6][i] = i % 2 === 0;
        matrix[i][6] = i % 2 === 0;
      }
      
      // 添加数据
      let hash = 0;
      for (let i = 0; i < data.length; i++) {
        hash = ((hash << 5) - hash) + data.charCodeAt(i);
      }
      
      for (let y = 0; y < size; y++) {
        for (let x = 0; x < size; x++) {
          if (!this.isReserved(x, y, size)) {
            matrix[y][x] = ((hash + x * y) % 3) === 0;
          }
        }
      }
      
      return matrix;
    }

    addFinderPattern(matrix, row, col) {
      for (let y = 0; y < 7; y++) {
        for (let x = 0; x < 7; x++) {
          if (y === 0 || y === 6 || x === 0 || x === 6 || 
              (y >= 2 && y <= 4 && x >= 2 && x <= 4)) {
            matrix[row + y][col + x] = true;
          }
        }
      }
    }

    isReserved(x, y, size) {
      // 检查是否是保留区域
      if (x < 9 && y < 9) return true;
      if (x < 9 && y >= size - 8) return true;
      if (x >= size - 8 && y < 9) return true;
      if (x === 6 || y === 6) return true;
      return false;
    }
  }

  // ================================
  // 主 Widget 类
  // ================================
  class GyIDWebButton {
    constructor(root, options) {
      this.root = typeof root === 'string' ? document.querySelector(root) : root;
      this.options = {
        position: 'bottom-right',
        locale: 'zh-CN',
        features: {
          fingerprint: true,
          geo: true,
          avatar: true,
          chain: true,
          qrExport: true
        },
        defaultChain: 'local',
        onGyIdGenerated: null,
        ...options
      };

      this.fingerprintCollector = new FingerprintCollector();
      this.geoService = new GeoService();
      this.avatarCollector = new AvatarCollector();
      this.gyIdGenerator = new GyIdGenerator();
      this.qrGenerator = new QRGenerator();

      this.state = {
        isOpen: false,
        geoPrecision: 'L1',
        chain: this.options.defaultChain,
        location: null,
        isGenerating: false,
        gyid: null
      };

      this.init();
    }

    init() {
      this.render();
      this.bindEvents();
      this.startDataCollection();
    }

    render() {
      const html = `
        <div class="gyid-widget ${this.options.position === 'bottom-left' ? 'gyid-widget-bottom-left' : ''}" id="gyid-widget">
          <!-- Tab 标签 -->
          <div class="gyid-widget-tab" id="gyid-tab">
            <span class="gyid-widget-dot"></span>
            <span class="gyid-widget-location" id="gyid-location">定位中...</span>
            <span class="gyid-widget-badge">GEOYUAN</span>
          </div>
          
          <!-- 展开面板 -->
          <div class="gyid-widget-panel" id="gyid-panel">
            <div class="gyid-widget-header">
              <div class="gyid-widget-title">
                ${Icons.location}
                <span>您的位置: <span id="gyid-panel-location">定位中...</span></span>
              </div>
              <button class="gyid-widget-close" id="gyid-close" title="收起">
                ${Icons.close}
              </button>
            </div>
            
            <div class="gyid-widget-body">
              <div class="gyid-generator">
                <p class="gyid-generator-desc">探索去中心化身份（DID），掌握您的数字主权。</p>
                
                <!-- 硬件指纹 -->
                ${this.options.features.fingerprint ? `
                <div class="gyid-section">
                  <div class="gyid-section-title">
                    <span class="gyid-section-icon-fingerprint">${Icons.fingerprint}</span>
                    硬件指纹
                  </div>
                  <div class="gyid-fingerprint-list" id="gyid-fingerprint-list">
                    <div class="gyid-fingerprint-item">
                      <span class="gyid-fingerprint-label">采集中...</span>
                    </div>
                  </div>
                </div>
                ` : ''}
                
                <!-- 地理位置 -->
                ${this.options.features.geo ? `
                <div class="gyid-section">
                  <div class="gyid-section-title">
                    <span class="gyid-section-icon-geo">${Icons.geo}</span>
                    地理位置
                  </div>
                  <div class="gyid-geo-options" id="gyid-geo-options">
                    <label class="gyid-geo-option selected" data-precision="L1">
                      <input type="radio" name="geo-precision" value="L1" checked>
                      <span class="gyid-radio"></span>
                      <div class="gyid-geo-content">
                        <div class="gyid-geo-label">仅城市 (L1)</div>
                        <div class="gyid-geo-desc">通过IP定位，适合一般场景</div>
                      </div>
                    </label>
                    <label class="gyid-geo-option" data-precision="L2">
                      <input type="radio" name="geo-precision" value="L2">
                      <span class="gyid-radio"></span>
                      <div class="gyid-geo-content">
                        <div class="gyid-geo-label">区域精度 (L2)</div>
                        <div class="gyid-geo-desc">IP定位 + 基站三角定位</div>
                      </div>
                    </label>
                    <label class="gyid-geo-option" data-precision="L3">
                      <input type="radio" name="geo-precision" value="L3">
                      <span class="gyid-radio"></span>
                      <div class="gyid-geo-content">
                        <div class="gyid-geo-label">GPS定位 (L3)</div>
                        <div class="gyid-geo-desc">GPS + WiFi 精确到米</div>
                      </div>
                    </label>
                  </div>
                  <div class="gyid-geo-location loading" id="gyid-geo-location">
                    <span class="gyid-geo-spinner"></span>
                    <span>正在获取位置...</span>
                  </div>
                </div>
                ` : ''}
                
                <!-- 头像上传 -->
                ${this.options.features.avatar ? `
                <div class="gyid-section">
                  <div class="gyid-section-title">
                    <span class="gyid-section-icon-avatar">${Icons.avatar}</span>
                    头像
                  </div>
                  <div class="gyid-avatar-upload">
                    <div class="gyid-avatar-preview" id="gyid-avatar-preview">
                      ${Icons.avatar}
                    </div>
                    <div class="gyid-avatar-info">
                      <div class="gyid-avatar-hint">支持 JPG、PNG，最大 5MB</div>
                      <button class="gyid-avatar-btn" id="gyid-upload-btn">
                        ${Icons.upload}
                        上传头像
                      </button>
                      <input type="file" class="gyid-avatar-input" id="gyid-avatar-input" accept="image/*">
                    </div>
                  </div>
                </div>
                ` : ''}
                
                <!-- 链上锚定 -->
                ${this.options.features.chain ? `
                <div class="gyid-section">
                  <div class="gyid-section-title">
                    <span class="gyid-section-icon-chain">${Icons.chain}</span>
                    链上锚定
                  </div>
                  <div class="gyid-chain-options" id="gyid-chain-options">
                    <div class="gyid-chain-option selected" data-chain="local">
                      <div class="gyid-chain-icon">${Icons.local}</div>
                      <div class="gyid-chain-name">仅本地</div>
                    </div>
                    <div class="gyid-chain-option" data-chain="polygon">
                      <div class="gyid-chain-icon">${Icons.polygon}</div>
                      <div class="gyid-chain-name">Polygon</div>
                    </div>
                    <div class="gyid-chain-option" data-chain="aptos">
                      <div class="gyid-chain-icon">${Icons.aptos}</div>
                      <div class="gyid-chain-name">Aptos</div>
                    </div>
                  </div>
                </div>
                ` : ''}
                
                <!-- 权重验证 -->
                <div class="gyid-weights" id="gyid-weights">
                  <div class="gyid-weights-title">权重验证</div>
                  <div class="gyid-weights-bar">
                    <div class="gyid-weights-bar-item">
                      <div class="gyid-weights-bar-fill" id="weight-fp" style="width: 0%"></div>
                    </div>
                    <div class="gyid-weights-bar-item">
                      <div class="gyid-weights-bar-fill" id="weight-geo" style="width: 0%"></div>
                    </div>
                    <div class="gyid-weights-bar-item">
                      <div class="gyid-weights-bar-fill" id="weight-avatar" style="width: 0%"></div>
                    </div>
                    <div class="gyid-weights-bar-item">
                      <div class="gyid-weights-bar-fill" id="weight-ts" style="width: 10%"></div>
                    </div>
                  </div>
                  <div class="gyid-weights-total invalid" id="gyid-weights-total">总计: 0% (需要 ≥60%)</div>
                </div>
                
                <!-- 生成按钮 -->
                <button class="gyid-generate-btn" id="gyid-generate-btn" disabled>
                  <span class="gyid-btn-text">${Icons.generate} 生成 GYID</span>
                  <span class="gyid-btn-loading">
                    <span class="gyid-geo-spinner"></span>
                    生成中...
                  </span>
                </button>
                
                <!-- 结果展示 -->
                <div class="gyid-result" id="gyid-result">
                  <div class="gyid-result-header">
                    <div class="gyid-result-title">
                      ${Icons.check}
                      GYID 生成成功
                    </div>
                    <button class="gyid-result-close" id="gyid-result-close">${Icons.close}</button>
                  </div>
                  <div class="gyid-id-display" id="gyid-id-display"></div>
                  <div class="gyid-qr-container">
                    <div class="gyid-qr-code" id="gyid-qr-code"></div>
                  </div>
                  ${this.options.features.qrExport ? `
                  <div class="gyid-result-actions">
                    <button class="gyid-result-btn" id="gyid-copy-btn">
                      ${Icons.copy} 复制
                    </button>
                    <button class="gyid-result-btn" id="gyid-export-btn">
                      ${Icons.download} 导出
                    </button>
                  </div>
                  ` : ''}
                </div>
              </div>
            </div>
          </div>
        </div>
      `;

      this.root.innerHTML = html;
      
      // 初始化组件引用
      this.widget = this.root.querySelector('#gyid-widget');
      this.tab = this.root.querySelector('#gyid-tab');
      this.panel = this.root.querySelector('#gyid-panel');
      this.closeBtn = this.root.querySelector('#gyid-close');
      
      // 设置头像预览引用
      if (this.options.features.avatar) {
        this.avatarCollector.setPreview(this.root.querySelector('#gyid-avatar-preview'));
      }
    }

    bindEvents() {
      // Tab 点击
      this.tab.addEventListener('click', () => this.toggle());

      // 关闭按钮
      this.closeBtn.addEventListener('click', () => this.close());

      // 地理位置选项
      const geoOptions = this.root.querySelectorAll('.gyid-geo-option');
      geoOptions.forEach(option => {
        option.addEventListener('click', () => {
          geoOptions.forEach(o => o.classList.remove('selected'));
          option.classList.add('selected');
          const precision = option.dataset.precision;
          this.state.geoPrecision = precision;
          this.geoService.setPrecision(precision);
          this.updateGeoLocation();
        });
      });

      // 链上锚定选项
      const chainOptions = this.root.querySelectorAll('.gyid-chain-option');
      chainOptions.forEach(option => {
        option.addEventListener('click', () => {
          chainOptions.forEach(o => o.classList.remove('selected'));
          option.classList.add('selected');
          this.state.chain = option.dataset.chain;
          this.updateWeights();
        });
      });

      // 头像上传
      const uploadBtn = this.root.querySelector('#gyid-upload-btn');
      const avatarInput = this.root.querySelector('#gyid-avatar-input');
      
      if (uploadBtn && avatarInput) {
        uploadBtn.addEventListener('click', () => avatarInput.click());
        avatarInput.addEventListener('change', async (e) => {
          if (e.target.files[0]) {
            try {
              await this.avatarCollector.handleFile(e.target.files[0]);
              this.updateWeights();
            } catch (error) {
              alert(error.message);
            }
          }
        });
      }

      // 生成按钮
      const generateBtn = this.root.querySelector('#gyid-generate-btn');
      if (generateBtn) {
        generateBtn.addEventListener('click', () => this.generateGyId());
      }

      // 结果关闭
      const resultClose = this.root.querySelector('#gyid-result-close');
      if (resultClose) {
        resultClose.addEventListener('click', () => {
          this.root.querySelector('#gyid-result').classList.remove('show');
        });
      }

      // 复制按钮
      const copyBtn = this.root.querySelector('#gyid-copy-btn');
      if (copyBtn) {
        copyBtn.addEventListener('click', () => this.copyGyId());
      }

      // 导出按钮
      const exportBtn = this.root.querySelector('#gyid-export-btn');
      if (exportBtn) {
        exportBtn.addEventListener('click', () => this.exportGyId());
      }
    }

    toggle() {
      if (this.state.isOpen) {
        this.close();
      } else {
        this.open();
      }
    }

    open() {
      this.state.isOpen = true;
      this.widget.classList.add('gyid-widget-visible');
      this.panel.classList.add('gyid-widget-panel-open');
    }

    close() {
      this.state.isOpen = false;
      this.widget.classList.remove('gyid-widget-visible');
      this.panel.classList.remove('gyid-widget-panel-open');
    }

    async startDataCollection() {
      // 收集指纹
      if (this.options.features.fingerprint) {
        await this.fingerprintCollector.collect();
        this.updateFingerprintDisplay();
      }

      // 获取位置
      this.updateGeoLocation();
    }

    updateFingerprintDisplay() {
      const list = this.root.querySelector('#gyid-fingerprint-list');
      if (!list) return;

      const summary = this.fingerprintCollector.getSummary();
      list.innerHTML = Object.entries(summary).map(([key, item]) => `
        <div class="gyid-fingerprint-item">
          <span class="gyid-fingerprint-label">
            ${item.label}: ${item.value}
          </span>
          <span class="gyid-fingerprint-status ${item.collected ? 'collected' : 'pending'}">
            ${Icons.check}
          </span>
        </div>
      `).join('');

      this.updateWeights();
    }

    async updateGeoLocation() {
      const locationEl = this.root.querySelector('#gyid-geo-location');
      const panelLocationEl = this.root.querySelector('#gyid-panel-location');
      const tabLocationEl = this.root.querySelector('#gyid-location');

      if (locationEl) {
        locationEl.classList.add('loading');
        locationEl.innerHTML = `
          <span class="gyid-geo-spinner"></span>
          <span>正在获取位置...</span>
        `;
      }

      try {
        const location = await this.geoService.getLocation();
        this.state.location = location;
        
        const displayText = this.geoService.getDisplayText();
        
        if (locationEl) {
          locationEl.classList.remove('loading');
          locationEl.textContent = displayText;
        }
        
        if (panelLocationEl) panelLocationEl.textContent = displayText;
        if (tabLocationEl) tabLocationEl.textContent = location.city;
        
        this.updateWeights();
      } catch (error) {
        console.error('位置获取失败:', error);
        if (locationEl) {
          locationEl.classList.remove('loading');
          locationEl.textContent = '定位失败，使用默认位置';
        }
      }
    }

    updateWeights() {
      const weights = {
        fingerprint: this.fingerprintCollector.data?.canvasHash ? 0.40 : 0,
        geo: this.state.location ? 0.35 : 0,
        avatar: this.avatarCollector.imageData ? 0.15 : 0,
        timestamp: 0.10
      };

      const total = weights.fingerprint + weights.geo + weights.avatar + weights.timestamp;

      // 更新进度条
      this.root.querySelector('#weight-fp').style.width = `${weights.fingerprint * 100}%`;
      this.root.querySelector('#weight-geo').style.width = `${weights.geo * 100}%`;
      this.root.querySelector('#weight-avatar').style.width = `${weights.avatar * 100}%`;
      this.root.querySelector('#weight-ts').style.width = `${weights.timestamp * 100}%`;

      // 更新总计
      const totalEl = this.root.querySelector('#gyid-weights-total');
      if (totalEl) {
        totalEl.textContent = `总计: ${(total * 100).toFixed(0)}% (需要 ≥60%)`;
        totalEl.classList.toggle('valid', total >= 0.6);
        totalEl.classList.toggle('invalid', total < 0.6);
      }

      // 更新按钮状态
      const generateBtn = this.root.querySelector('#gyid-generate-btn');
      if (generateBtn) {
        generateBtn.disabled = total < 0.6;
      }
    }

    async generateGyId() {
      if (this.state.isGenerating) return;

      this.state.isGenerating = true;
      const generateBtn = this.root.querySelector('#gyid-generate-btn');
      const result = this.root.querySelector('#gyid-result');
      
      generateBtn.classList.add('loading');

      try {
        // 准备数据
        const data = {
          fingerprint: this.fingerprintCollector.data,
          geo: this.state.location,
          avatar: await this.avatarCollector.getHash(),
          chain: this.state.chain
        };

        this.gyIdGenerator.setData(data);

        // 生成 GYID (模拟延迟)
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        const gyidResult = this.gyIdGenerator.generate();
        this.state.gyid = gyidResult;

        // 显示结果
        const idDisplay = this.root.querySelector('#gyid-id-display');
        const qrCode = this.root.querySelector('#gyid-qr-code');
        
        idDisplay.textContent = gyidResult.gyid;
        
        // 生成二维码
        await this.qrGenerator.generate(qrCode, gyidResult.qrData);
        
        result.classList.add('show');

        // 回调
        if (this.options.onGyIdGenerated) {
          this.options.onGyIdGenerated(gyidResult);
        }

      } catch (error) {
        console.error('GYID 生成失败:', error);
        alert('生成失败: ' + error.message);
      } finally {
        this.state.isGenerating = false;
        generateBtn.classList.remove('loading');
      }
    }

    async copyGyId() {
      if (!this.state.gyid) return;

      try {
        await navigator.clipboard.writeText(this.state.gyid.gyid);
        alert('GYID 已复制到剪贴板');
      } catch (error) {
        console.error('复制失败:', error);
        alert('复制失败，请手动复制');
      }
    }

    async exportGyId() {
      if (!this.state.gyid) return;

      const data = {
        gyid: this.state.gyid.gyid,
        createdAt: this.state.gyid.createdAt,
        weights: this.state.gyid.weights,
        chain: this.state.chain
      };

      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `gyid-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    }

    static init(options = {}) {
      const root = document.getElementById('gyid-widget-root') || document.body;
      return new GyIDWebButton(root, options);
    }
  }

  // 导出到全局
  global.GyIDWebButton = GyIDWebButton;

})(typeof window !== 'undefined' ? window : this);
