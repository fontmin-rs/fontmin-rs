import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'fontmin-rs',
  description:
    'Fast font subsetting and conversion tooling written in Rust with Node.js bindings.',
  srcExclude: ['superpowers/**/*.md'],
  cleanUrls: true,
  lastUpdated: true,
  markdown: {
    lineNumbers: true,
  },
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      title: 'fontmin-rs',
      description:
        'Fast font subsetting and conversion tooling written in Rust with Node.js bindings.',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/guide/getting-started' },
          { text: 'API', link: '/api/node' },
          { text: 'Architecture', link: '/architecture' },
        ],
        sidebar: [
          {
            text: 'Getting Started',
            items: [
              { text: 'Quick Start', link: '/guide/getting-started' },
              { text: 'Command Line', link: '/guide/cli' },
              { text: 'Configuration', link: '/guide/config' },
              { text: 'Migration From Fontmin', link: '/guide/migration' },
            ],
          },
          {
            text: 'Reference',
            items: [
              { text: 'Node API', link: '/api/node' },
              { text: 'Architecture', link: '/architecture' },
            ],
          },
        ],
        editLink: {
          pattern: 'https://github.com/ntnyq/fontmin-rs/edit/main/docs/:path',
          text: 'Edit this page on GitHub',
        },
        lastUpdated: {
          text: 'Last updated',
        },
        outline: {
          level: [2, 3],
          label: 'On this page',
        },
      },
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'fontmin-rs',
      description:
        '基于 Rust 与 Node.js binding 的高性能字体子集化和格式转换工具。',
      themeConfig: {
        nav: [
          { text: '指南', link: '/zh/guide/getting-started' },
          { text: 'API', link: '/zh/api/node' },
          { text: '架构', link: '/zh/architecture' },
        ],
        sidebar: [
          {
            text: '开始使用',
            items: [
              { text: '快速开始', link: '/zh/guide/getting-started' },
              { text: '命令行', link: '/zh/guide/cli' },
              { text: '配置文件', link: '/zh/guide/config' },
              { text: '从 Fontmin 迁移', link: '/zh/guide/migration' },
            ],
          },
          {
            text: '参考',
            items: [
              { text: 'Node API', link: '/zh/api/node' },
              { text: '项目架构', link: '/zh/architecture' },
            ],
          },
        ],
        editLink: {
          pattern: 'https://github.com/ntnyq/fontmin-rs/edit/main/docs/:path',
          text: '在 GitHub 上编辑此页',
        },
        lastUpdated: {
          text: '最后更新',
        },
        outline: {
          level: [2, 3],
          label: '本页目录',
        },
      },
    },
  },
  themeConfig: {
    logo: '/logo.svg',
    search: {
      provider: 'local',
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
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/ntnyq/fontmin-rs' },
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-PRESENT ntnyq',
    },
  },
})
