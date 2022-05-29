FROM node:12
WORKDIR /app
COPY ./app .
RUN yarn install && yarn build

EXPOSE 3000
CMD ["yarn", "start"]
