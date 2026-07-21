export const AppHeader = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex h-16 w-full items-center justify-between border-b border-b-slate-200 bg-white px-4 text-gray-700">
      {children}
    </div>
  )
}
