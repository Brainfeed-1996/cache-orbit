FROM node:20-bullseye-slim AS builder
WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

FROM node:20-bullseye-slim
WORKDIR /app

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public
COPY --from=builder /app/node_modules ./node_modules

EXPOSE 3000

ENV NODE_ENV=production
HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl -f http://localhost:3000/ || exit 1

CMD ["node", "server.js"]
