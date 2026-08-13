import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from '@apollo/client/react'
import { LayoutGrid, ArrowRight } from 'lucide-react'
import { useAuthStore } from '@/stores/auth'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { GET_SPACES, GET_SPACE_STATS, TEST_SPACE_ID } from '@/api/spaces'
import type { Space, SpaceStats } from '@/api/spaces'

interface Feature {
  title: string
  subtitle: string
  description: string
  to: string
  icon: ReactNode
}

const businessFeatures: Feature[] = [
  {
    title: '价值流',
    subtitle: 'Value Streams',
    description: '梳理端到端价值交付流程，识别增值与非增值环节，驱动业务持续优化。',
    to: `/spaces/${TEST_SPACE_ID}/architectures/value-streams`,
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
    to: `/spaces/${TEST_SPACE_ID}/architectures/capabilities`,
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
    to: `/spaces/${TEST_SPACE_ID}/architectures/processes`,
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

const applicationFeatures: Feature[] = [
  {
    title: '应用组件',
    subtitle: 'Application Components',
    description: '管理应用系统的组成单元与交付物，明确系统边界与实现载体。',
    to: `/spaces/${TEST_SPACE_ID}/architectures/applications`,
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
        <rect x="3" y="3" width="18" height="7" rx="1" />
        <rect x="3" y="14" width="18" height="7" rx="1" />
      </svg>
    ),
  },
  {
    title: '应用流程',
    subtitle: 'Application Processes',
    description: '定义应用系统的运行流程与自动化任务，支撑业务流程落地。',
    to: `/spaces/${TEST_SPACE_ID}/architectures/application-processes`,
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
        <circle cx="5" cy="6" r="3" />
        <circle cx="19" cy="6" r="3" />
        <path d="M5 9v12" />
        <path d="M19 9v3" />
        <path d="M5 15h14" />
      </svg>
    ),
  },
  {
    title: '功能模块',
    subtitle: 'Functional Modules',
    description: '划分应用功能边界，建立模块与组件的包含关系，实现架构分层。',
    to: `/spaces/${TEST_SPACE_ID}/architectures/functional-modules`,
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
        <rect x="2" y="2" width="20" height="8" rx="1" />
        <rect x="2" y="14" width="20" height="8" rx="1" />
      </svg>
    ),
  },
  {
    title: '应用接口',
    subtitle: 'Application Interfaces',
    description: '定义应用间的接口契约与数据交换，保障系统集成与协同。',
    to: `/spaces/${TEST_SPACE_ID}/architectures/application-interfaces`,
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
        <circle cx="12" cy="5" r="3" />
        <circle cx="5" cy="19" r="3" />
        <circle cx="19" cy="19" r="3" />
        <path d="M12 8v8" />
        <path d="M7 19h10" />
      </svg>
    ),
  },
]

function FeatureCard({ feature }: { feature: Feature }) {
  return (
    <Link to={feature.to} className="block h-full">
      <Card className="h-full hover:shadow-md transition-shadow">
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
    </Link>
  )
}

export default function Home() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  const { data: spacesData, loading: spacesLoading, error: spacesError } = useQuery<{
    spaces: Space[]
  }>(GET_SPACES)
  const { data: statsData } = useQuery<SpaceStats>(GET_SPACE_STATS, {
    variables: { spaceId: TEST_SPACE_ID },
  })

  const spaces = spacesData?.spaces ?? []
  const stats = [
    { label: '空间', value: spaces.length },
    { label: '价值流', value: statsData?.valueStreamCountBySpace ?? 0 },
    { label: '业务能力', value: statsData?.businessCapabilityCountBySpace ?? 0 },
    { label: '业务流程', value: statsData?.businessProcessCountBySpace ?? 0 },
  ]

  return (
    <div className="min-h-screen bg-secondary flex flex-col">
      <header className="border-b bg-background">
        <div className="container mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <span className="text-lg font-semibold">企业架构平台</span>
          <Link to={isAuthenticated ? '/spaces' : '/login'}>
            <Button variant={isAuthenticated ? 'default' : 'outline'}>
              {isAuthenticated ? '进入平台' : '登录'}
            </Button>
          </Link>
        </div>
      </header>

      <main className="flex-1">
        <section className="container mx-auto max-w-6xl px-4 py-16 md:py-24 text-center">
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight">
            企业架构平台
          </h1>
          <p className="mt-2 text-base md:text-lg text-muted-foreground">
            从战略到执行的企业架构建模与协同平台
          </p>
          <p className="mx-auto mt-6 max-w-2xl text-base md:text-lg text-muted-foreground">
            一体化的企业架构建模与协同平台，帮助您梳理价值流、规划业务能力、编排业务流程，
            实现战略对齐、端到端可视化与决策支撑。
          </p>
          <div className="mt-8 flex items-center justify-center gap-4 flex-wrap">
            <Link to="/spaces">
              <Button size="lg">
                浏览架构空间
              </Button>
            </Link>
            {!isAuthenticated && (
              <Link to="/login">
                <Button size="lg" variant="outline">
                  登录
                </Button>
              </Link>
            )}
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
            平台能力
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            业务架构与应用架构双域协同，覆盖企业架构的核心场景
          </p>
          <h3 className="mt-10 text-lg font-semibold tracking-tight">业务架构</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            从战略到执行的业务建模：价值流、业务能力与业务流程
          </p>
          <div className="mt-4 grid gap-6 md:grid-cols-3">
            {businessFeatures.map((feature) => (
              <FeatureCard key={feature.title} feature={feature} />
            ))}
          </div>
          <h3 className="mt-10 text-lg font-semibold tracking-tight">应用架构</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            支撑业务的系统实现：应用组件、应用流程、功能模块与应用接口
          </p>
          <div className="mt-4 grid gap-6 md:grid-cols-2 lg:grid-cols-4">
            {applicationFeatures.map((feature) => (
              <FeatureCard key={feature.title} feature={feature} />
            ))}
          </div>
        </section>

        <section className="container mx-auto max-w-6xl px-4 pb-20 md:pb-28">
          <h2 className="text-center text-2xl md:text-3xl font-semibold tracking-tight">
            架构概览
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            来自真实企业架构案例的数据
          </p>

          <div className="mt-10 grid grid-cols-2 gap-4 md:grid-cols-4">
            {stats.map((item) => (
              <Card key={item.label}>
                <CardContent className="p-6 text-center">
                  <p className="text-3xl font-bold">{item.value}</p>
                  <p className="mt-1 text-sm text-muted-foreground">{item.label}</p>
                </CardContent>
              </Card>
            ))}
          </div>

          {spacesLoading && (
            <div className="mt-10 text-center text-muted-foreground">加载中...</div>
          )}
          {spacesError && (
            <div className="mt-10 text-center text-destructive">加载失败: {spacesError.message}</div>
          )}

          <div className="mt-10 grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {spaces.map((space) => (
              <Link key={space.id} to={`/spaces/${space.id}`}>
                <Card className="h-full hover:shadow-md transition-shadow">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      <LayoutGrid className="h-4 w-4 text-muted-foreground" />
                      {space.name}
                    </CardTitle>
                    <CardDescription>
                      {space.description || '暂无描述'}
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <p className="text-xs text-muted-foreground">
                      创建于 {new Date(space.createdAt).toLocaleDateString()}
                    </p>
                  </CardContent>
                </Card>
              </Link>
            ))}
            {!spacesLoading && !spacesError && spaces.length === 0 && (
              <div className="col-span-full text-center py-16 text-muted-foreground">
                暂无空间
              </div>
            )}
          </div>

          {!spacesLoading && !spacesError && spaces.length > 0 && (
            <div className="mt-10 text-center">
              <Link to="/spaces">
                <Button variant="outline">
                  查看全部空间
                  <ArrowRight className="h-4 w-4 ml-2" />
                </Button>
              </Link>
            </div>
          )}
        </section>
      </main>

      <footer className="border-t bg-background">
        <div className="container mx-auto max-w-6xl px-4 py-6 text-center text-sm text-muted-foreground">
          © {new Date().getFullYear()} 企业架构平台 · 个人技术项目
          <a href="https://beian.miit.gov.cn" target="_blank" rel="noopener noreferrer" className="hover:text-foreground ml-2">
            粤ICP备2025471124号
          </a>
        </div>
      </footer>
    </div>
  )
}