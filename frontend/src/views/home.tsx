import { Link } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const features = [
  {
    title: '价值流',
    subtitle: 'Value Streams',
    description: '梳理端到端价值交付流程，识别增值与非增值环节，驱动业务持续优化。',
    icon: (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="h-6 w-6"
      >
        <path d="M5 12h14" />
        <path d="M12 5l7 7-7 7" />
      </svg>
    ),
  },
  {
    title: '业务能力',
    subtitle: 'Business Capabilities',
    description: '结构化描述组织核心能力，建立能力地图，支撑战略规划与资源配置。',
    icon: (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="h-6 w-6"
      >
        <rect x="3" y="3" width="7" height="7" rx="1" />
        <rect x="14" y="3" width="7" height="7" rx="1" />
        <rect x="3" y="14" width="7" height="7" rx="1" />
        <rect x="14" y="14" width="7" height="7" rx="1" />
      </svg>
    ),
  },
  {
    title: '业务流程',
    subtitle: 'Business Processes',
    description: '定义并管理业务流程与活动，串联能力与价值流，实现流程可视化与协同。',
    icon: (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="h-6 w-6"
      >
        <path d="M6 3v12" />
        <circle cx="6" cy="18" r="3" />
        <path d="M18 3v6" />
        <circle cx="18" cy="12" r="3" />
        <path d="M6 9h12" />
      </svg>
    ),
  },
]

export default function Home() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  return (
    <div className="min-h-screen bg-secondary flex flex-col">
      <header className="border-b bg-background">
        <div className="container mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <span className="text-lg font-semibold">技术实验与学习成果展示</span>
          <Link to={isAuthenticated ? '/architectures/value-streams' : '/login'}>
            <Button variant={isAuthenticated ? 'default' : 'outline'}>
              {isAuthenticated ? '进入平台' : '登录'}
            </Button>
          </Link>
        </div>
      </header>

      <main className="flex-1">
        <section className="container mx-auto max-w-6xl px-4 py-16 md:py-24 text-center">
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight">
            技术实验与学习成果展示
          </h1>
          <p className="mt-2 text-base md:text-lg text-muted-foreground">
            个人技术项目展示 · Rust + React 全栈应用
          </p>
          <p className="mx-auto mt-6 max-w-2xl text-base md:text-lg text-muted-foreground">
            一体化的企业架构建模与管理平台，帮助您梳理价值流、规划业务能力、编排业务流程，
            实现从战略到执行的端到端可视化与协同。
          </p>
          <div className="mt-8 flex items-center justify-center gap-4">
            <Link to={isAuthenticated ? '/architectures/value-streams' : '/login'}>
              <Button size="lg">
                {isAuthenticated ? '进入平台' : '登录'}
              </Button>
            </Link>
            <a
              href="#features"
              className="inline-flex h-11 items-center justify-center rounded-md border border-input bg-background px-8 text-sm font-medium hover:bg-accent hover:text-accent-foreground"
            >
              了解更多
            </a>
          </div>
        </section>

        <section
          id="features"
          className="container mx-auto max-w-6xl px-4 pb-20 md:pb-28"
        >
          <h2 className="text-center text-2xl md:text-3xl font-semibold tracking-tight">
            核心模块
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            三大模块协同工作，覆盖企业架构的核心场景
          </p>
          <div className="mt-10 grid gap-6 md:grid-cols-3">
            {features.map((feature) => (
              <Card key={feature.title} className="flex flex-col">
                <CardHeader>
                  <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10 text-primary">
                    {feature.icon}
                  </div>
                  <CardTitle className="mt-4">{feature.title}</CardTitle>
                  <CardDescription>{feature.subtitle}</CardDescription>
                </CardHeader>
                <CardContent className="flex-1">
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {feature.description}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>
      </main>

      <footer className="border-t bg-background">
        <div className="container mx-auto max-w-6xl px-4 py-6 text-center text-sm text-muted-foreground">
          © {new Date().getFullYear()} 技术实验与学习成果展示 · 个人技术项目
          <a href="https://beian.miit.gov.cn" target="_blank" rel="noopener noreferrer" className="hover:text-foreground ml-2">
            粤ICP备2025471124号
          </a>
        </div>
      </footer>
    </div>
  )
}