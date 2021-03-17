FROM node:15.7.0-buster

# make the 'app' folder the current working directory
WORKDIR /app

# copy both 'package.json' and 'package-lock.json' (if available)
# (note that package-lock.json is for npm, and we use yarn here, so not super useful)
COPY package.json ./ 
COPY yarn.lock .

# install project dependencies
RUN yarn global add @vue/cli
RUN yarn install

# copy project files and folders to the current working directory (i.e. 'app' folder)
COPY . .

EXPOSE 8080
CMD [ "yarn", "serve" ]
