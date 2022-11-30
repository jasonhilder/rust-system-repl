if [ -f /package.json ]; then
    chmod 777 package.json
    mv /package.json /rusty-rep
fi

if [ -f /main.js ]; then
    chmod 777 main.js
    mv /main.js /rusty-rep
fi
