import { fileURLToPath, URL } from 'node:url'
import UnoCSS from 'unocss/vite'
import { defineConfig } from 'vitepress'

export default defineConfig({
  cleanUrls: true,
  description:
    'Fast font subsetting and conversion tooling written in Rust with Node.js bindings.',
  lastUpdated: true,
  locales: {
    root: {
      description:
        'Fast font subsetting and conversion tooling written in Rust with Node.js bindings.',
      label: 'English',
      lang: 'en-US',
      themeConfig: {
        editLink: {
          pattern:
            'https://github.com/fontmin-rs/fontmin-rs/edit/main/docs/:path',
          text: 'Edit this page on GitHub',
        },
        lastUpdated: {
          text: 'Last updated',
        },
        nav: [
          { link: '/guide/getting-started', text: 'Guide' },
          { link: '/api/node', text: 'API' },
          { link: '/playground', text: 'Playground' },
          { link: '/architecture', text: 'Architecture' },
          { link: '/roadmap', text: 'Roadmap' },
        ],
        outline: {
          label: 'On this page',
          level: [2, 3],
        },
        sidebar: [
          {
            items: [
              { link: '/guide/getting-started', text: 'Quick Start' },
              { link: '/guide/features', text: 'Features' },
              { link: '/guide/cli', text: 'Command Line' },
              { link: '/guide/config', text: 'Configuration' },
              { link: '/guide/migration', text: 'Migration From Fontmin' },
            ],
            text: 'Getting Started',
          },
          {
            items: [
              { link: '/api/node', text: 'Node API' },
              { link: '/api/wasm', text: 'Browser WASM API' },
              { link: '/architecture', text: 'Architecture' },
              { link: '/roadmap', text: 'Roadmap to 1.0' },
            ],
            text: 'Reference',
          },
        ],
      },
      title: 'fontmin-rs',
    },
    zh: {
      description:
        '基于 Rust 与 Node.js binding 的高性能字体子集化和格式转换工具。',
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      themeConfig: {
        editLink: {
          pattern:
            'https://github.com/fontmin-rs/fontmin-rs/edit/main/docs/:path',
          text: '在 GitHub 上编辑此页',
        },
        lastUpdated: {
          text: '最后更新',
        },
        nav: [
          { link: '/zh/guide/getting-started', text: '指南' },
          { link: '/zh/api/node', text: 'API' },
          { link: '/zh/playground', text: 'Playground' },
          { link: '/zh/architecture', text: '架构' },
          { link: '/zh/roadmap', text: '路线图' },
        ],
        outline: {
          label: '本页目录',
          level: [2, 3],
        },
        sidebar: [
          {
            items: [
              { link: '/zh/guide/getting-started', text: '快速开始' },
              { link: '/zh/guide/features', text: '功能概览' },
              { link: '/zh/guide/cli', text: '命令行' },
              { link: '/zh/guide/config', text: '配置文件' },
              { link: '/zh/guide/migration', text: '从 Fontmin 迁移' },
            ],
            text: '开始使用',
          },
          {
            items: [
              { link: '/zh/api/node', text: 'Node API' },
              { link: '/zh/api/wasm', text: '浏览器 WASM API' },
              { link: '/zh/architecture', text: '项目架构' },
              { link: '/zh/roadmap', text: '迈向 1.0' },
            ],
            text: '参考',
          },
        ],
      },
      title: 'fontmin-rs',
    },
  },
  markdown: {
    lineNumbers: true,
  },
  srcExclude: ['superpowers/**/*.md'],
  themeConfig: {
    footer: {
      copyright: 'Copyright © 2026-PRESENT ntnyq',
      message: 'Released under the MIT License.',
    },
    logo: '/logo.svg',
    search: {
      options: {
        locales: {
          zh: {
            translations: {
              button: {
                buttonAriaLabel: '搜索',
                buttonText: '搜索',
              },
              modal: {
                displayDetails: '显示详情',
                footer: {
                  closeKeyAriaLabel: '关闭',
                  closeText: '关闭',
                  navigateDownKeyAriaLabel: '向下',
                  navigateText: '切换',
                  navigateUpKeyAriaLabel: '向上',
                  selectKeyAriaLabel: '选择',
                  selectText: '选择',
                },
                noResultsText: '没有找到结果',
                resetButtonTitle: '重置搜索',
              },
            },
          },
        },
      },
      provider: 'local',
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/fontmin-rs/fontmin-rs' },
    ],
  },
  title: 'fontmin-rs',
  vite: {
    plugins: [
      UnoCSS({
        inspector: false,
      }),
    ],
    resolve: {
      alias: {
        '@fontmin-rs/wasm': fileURLToPath(
          new URL('../../wasm/fontmin/src/index.ts', import.meta.url),
        ),
      },
    },
  },
})
