# download ubuntu cloud init image as a base for the test harness

# place it locally in the test harness folder
mkdir -p "../artefacts/"
cd ../artefacts/ || exit

# do not assume the user has already downloaded this specific cloud init image via running the testbed
wget -nc https://cloud-images.ubuntu.com/focal/current/focal-server-cloudimg-amd64.img
