/**
 * PostCSS Configuration
 *
 * Interview Q&A:
 *
 * Q: PostCSS의 역할은?
 * A: CSS 트랜스폼 파이프라인
 *    1. tailwindcss: Utility 클래스 생성
 *    2. autoprefixer: 벤더 프리픽스 자동 추가
 *    → 브라우저 호환성 보장
 */

export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
