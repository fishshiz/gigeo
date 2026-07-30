export function getRandomPlaylistName(city: string) {
  const templates = [
    `Live, from ${city}`,
    `Tonight in ${city}`,
    `${city} After Dark`,
    `Sounds of ${city}`,
    `On Stage in ${city}`,
    `${city} Nightlights`,
    `From the Heart of ${city}`,
    `${city}, Amplified`,
    `Late Nights in ${city}`,
    `Live and Loud in ${city}`,
  ]

  return city.length > 0
    ? templates[Math.floor(Math.random() * templates.length)]
    : ""
}
