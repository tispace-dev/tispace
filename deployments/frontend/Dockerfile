# Install dependencies only when needed
FROM node:16-alpine AS deps

WORKDIR /opt/app
COPY ./app/package.json ./app/yarn.lock ./
RUN yarn install --frozen-lockfile

# Rebuild the source code only when needed
FROM node:16-alpine AS builder

ENV NODE_ENV=production
ENV NEXT_PUBLIC_SERVER_URL=https://api.tispace.dev
WORKDIR /opt/app
COPY ./app/ .
COPY --from=deps /opt/app/node_modules ./node_modules
RUN yarn build

# Production image, copy all the files and run next
FROM node:16-alpine AS runner

WORKDIR /opt/app
ENV NODE_ENV=production
COPY --from=builder /opt/app/next.config.js ./
COPY --from=builder /opt/app/public ./public
COPY --from=builder /opt/app/.next ./.next
COPY --from=builder /opt/app/node_modules ./node_modules
CMD ["node_modules/.bin/next", "start"]
