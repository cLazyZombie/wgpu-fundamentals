# WGPU 삼각형 그리기

이 예제는 WGPU를 사용해서 컬러풀한 삼각형을 그립니다.

<!-- <div id="triangle-demo" style="text-align: center; margin: 20px 0;"> -->
<!--     <canvas id="triangle-canvas" style="border: 1px solid #ccc;"></canvas> -->
<!--     <br> -->
<!--     <button onclick="runWGPUTriangle('triangle-canvas')" style="margin-top: 10px; padding: 10px 20px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;"> -->
<!--         삼각형 그리기 -->
<!--     </button> -->
<!-- </div> -->
<!---->
<!-- <script> -->
<!-- // 페이지 로드 시 자동 실행 (선택사항) -->
<!-- document.addEventListener('DOMContentLoaded', function() { -->
<!--     // 자동으로 실행하려면 주석 해제 -->
<!--     // setTimeout(() => runWGPUTriangle('triangle-canvas'), 1000); -->
<!-- }); -->
<!-- </script> -->
<div id="wgpu-demo-container" style="text-align: center; margin: 20px 0;">
    <canvas id="wgpu-canvas" width="600" height="400" style="border: 2px solid #333; background: white; max-width: 100%;"></canvas>
    <div id="loading" style="margin-top: 10px;">Loading WebAssembly...</div>
    <div id="error" style="margin-top: 10px; color: red; display: none;"></div>
</div>

<script type="module">
    async function initWgpu() {
        const loadingDiv = document.getElementById('loading');
        const errorDiv = document.getElementById('error');
        
        try {
            // WebAssembly 모듈 동적 로드
            const wasmModule = await import('./assets/wasm/wgpu-triangle/wgpu_triangle.js');
            
            // wasm 초기화
            await wasmModule.default();
            
            // 캔버스에서 렌더링 시작
            await wasmModule.run('wgpu-canvas');
            
            loadingDiv.style.display = 'none';
            console.log('wgpu initialized successfully!');
            
        } catch (error) {
            console.error('Failed to initialize wgpu:', error);
            loadingDiv.style.display = 'none';
            errorDiv.style.display = 'block';
            errorDiv.textContent = `Error: ${error.message}`;
        }
    }
    
    // 페이지 로드 후 초기화
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initWgpu);
    } else {
        initWgpu();
    }
</script>
## 코드 설명

위의 삼각형은 다음과 같은 과정으로 그려집니다:

1. **WGPU 초기화**: WebGPU 인스턴스와 디바이스 생성
2. **셰이더 컴파일**: WGSL로 작성된 vertex/fragment 셰이더
3. **버텍스 데이터**: 삼각형의 3개 꼭짓점과 색상 정보
4. **렌더 파이프라인**: 렌더링 설정 및 파이프라인 구성
5. **그리기**: 실제 렌더링 수행

```rust
// 버텍스 데이터 예시
const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
];
