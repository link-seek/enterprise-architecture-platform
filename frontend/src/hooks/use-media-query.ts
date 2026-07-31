import { useState, useEffect } from 'react'

/**
 * Returns true when the viewport is narrower than `breakpoint` (default 768,
 * matching Tailwind's `md`). Uses matchMedia for efficient change detection.
 * Initial value is computed synchronously to minimize first-paint flicker.
 */
export function useIsMobile(breakpoint = 768): boolean {
  const [isMobile, setIsMobile] = useState(
    typeof window !== 'undefined' ? window.innerWidth < breakpoint : false
  )

  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${breakpoint - 1}px)`)
    const handler = () => setIsMobile(mql.matches)
    handler()
    mql.addEventListener('change', handler)
    return () => mql.removeEventListener('change', handler)
  }, [breakpoint])

  return isMobile
}