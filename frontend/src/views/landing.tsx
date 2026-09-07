import type { ReactNode } from 'react'
import { Link, Navigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { TEST_SPACE_ID } from '@/api/spaces'

interface Topic {
  title: string
  subtitle: string
  description: string
  to: string
  icon: ReactNode
}

const learningTopics: Topic[] = [
  {
    title: '价值流',
    subtitle: 'Value Streams',
    description: '记录端到端价值交付流程的学习与拆解，梳理增值与非增值环节。',
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
    description: '结构化梳理组织核心能力，建立能力地图的学习笔记。',
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
    description: '学习并记录业务流程与活动，串联能力与价值流。',
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

const practiceTopics: Topic[] = [
  {
    title: '应用组件',
    subtitle: 'Application Components',
    description: '记录应用系统的组成单元与交付物，明确系统边界。',
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
    description: '记录应用系统的运行流程与自动化任务的实践。',
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
    description: '划分应用功能边界，记录模块与组件的分层关系。',
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
    description: '记录应用间的接口契约与数据交换的学习心得。',
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

function TopicCard({ topic }: { topic: Topic }) {
  return (
    <Link to={topic.to} className="block h-full">
      <Card className="h-full hover:shadow-md transition-shadow">
        <CardHeader>
          <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10 text-primary">
            {topic.icon}
          </div>
          <CardTitle className="mt-4">{topic.title}</CardTitle>
          <CardDescription>{topic.subtitle}</CardDescription>
        </CardHeader>
        <CardContent className="flex-1">
          <p className="text-sm text-muted-foreground leading-relaxed">
            {topic.description}
          </p>
        </CardContent>
      </Card>
    </Link>
  )
}

export default function Landing() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  if (isAuthenticated) {
    return <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/overview`} replace />
  }

  return (
    <div className="min-h-screen bg-secondary flex flex-col">
      <header className="border-b bg-background">
        <div className="container mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <span className="text-lg font-semibold">个人技术学习记录</span>
          <Link to="/login">
            <Button variant="outline">登录</Button>
          </Link>
        </div>
      </header>

      <main className="flex-1">
        <section className="container mx-auto max-w-6xl px-4 py-16 md:py-24 text-center">
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight">
            个人技术学习记录
          </h1>
          <p className="mt-2 text-base md:text-lg text-muted-foreground">
            架构建模学习与实践的个人笔记空间
          </p>
          <p className="mx-auto mt-6 max-w-2xl text-base md:text-lg text-muted-foreground">
            记录企业架构建模的学习方向、实践项目与复盘思考，
            梳理价值流、业务能力与业务流程，沉淀个人技术成长。
          </p>
          <div className="mt-8 flex items-center justify-center gap-4 flex-wrap">
            <Link to="/spaces">
              <Button size="lg">浏览记录</Button>
            </Link>
            <Link to="/login">
              <Button size="lg" variant="outline">登录</Button>
            </Link>
          </div>
        </section>

        <section className="container mx-auto max-w-6xl px-4 pb-20 md:pb-28">
          <h2 className="text-center text-2xl md:text-3xl font-semibold tracking-tight">
            学习方向
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            业务架构学习笔记：价值流、业务能力与业务流程
          </p>
          <div className="mt-8 grid gap-6 md:grid-cols-3">
            {learningTopics.map((topic) => (
              <TopicCard key={topic.title} topic={topic} />
            ))}
          </div>
        </section>

        <section className="container mx-auto max-w-6xl px-4 pb-20 md:pb-28">
          <h2 className="text-center text-2xl md:text-3xl font-semibold tracking-tight">
            实践项目
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            应用架构实践记录：应用组件、应用流程、功能模块与应用接口
          </p>
          <div className="mt-8 grid gap-6 md:grid-cols-2 lg:grid-cols-4">
            {practiceTopics.map((topic) => (
              <TopicCard key={topic.title} topic={topic} />
            ))}
          </div>
        </section>

        <section className="container mx-auto max-w-6xl px-4 pb-20 md:pb-28">
          <h2 className="text-center text-2xl md:text-3xl font-semibold tracking-tight">
            复盘
          </h2>
          <p className="mt-3 text-center text-muted-foreground">
            架构总览与映射关系，沉淀学习心得
          </p>
          <div className="mt-8 grid gap-6 md:grid-cols-2">
            <Link to={`/spaces/${TEST_SPACE_ID}/architectures/overview`} className="block h-full">
              <Card className="h-full hover:shadow-md transition-shadow">
                <CardHeader>
                  <CardTitle>架构总览</CardTitle>
                  <CardDescription>Architecture Overview</CardDescription>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    汇总各域架构建模成果，全局视角复盘学习进展。
                  </p>
                </CardContent>
              </Card>
            </Link>
            <Link to={`/spaces/${TEST_SPACE_ID}/architectures/realizations`} className="block h-full">
              <Card className="h-full hover:shadow-md transition-shadow">
                <CardHeader>
                  <CardTitle>映射关系</CardTitle>
                  <CardDescription>Realizations</CardDescription>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    业务能力到流程的映射，梳理跨域关联与实现路径。
                  </p>
                </CardContent>
              </Card>
            </Link>
          </div>
        </section>
      </main>

      <footer className="border-t bg-background">
        <div className="container mx-auto max-w-6xl px-4 py-6 text-center text-sm text-muted-foreground">
          © {new Date().getFullYear()} 个人技术项目
          <a href="https://beian.miit.gov.cn" target="_blank" rel="noopener noreferrer" className="hover:text-foreground ml-2">
            粤ICP备2025471124号
          </a>
        </div>
      </footer>
    </div>
  )
}