import * as React from 'react'
import { Box } from '../../ink.js'
import { Clawd } from './Clawd.js'

const SAI_HEIGHT = 6

export function AnimatedClawd(): React.ReactNode {
  return (
    <Box height={SAI_HEIGHT} flexDirection="column">
      <Clawd />
    </Box>
  )
}
