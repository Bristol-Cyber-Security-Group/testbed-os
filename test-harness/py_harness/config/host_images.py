import enum
import logging
from py_harness import harness_settings
from pathlib import Path
from urllib.request import urlretrieve


class OSInit(enum.Enum):

    cloud_init = 0


class OSData:

    filename: str
    download_url: str
    os_variant: str
    # hash: str
    os_init: OSInit

    def __init__(self, filename: str, download_url: str, os_variant: str, os_init: OSInit):
        self.filename = filename
        self.download_url = download_url
        self.os_variant = os_variant
        self.os_init = os_init

@enum.unique
class BaseOperatingSystem(enum.Enum):
    """
    This enum represents the range of operating systems that we are testing against. The corresponding value links to
    the latest version download of the disk image, ready to be used by libvirt.
    """

    # Ubuntu20_04 = OSData(
    #     filename="focal-server-cloudimg-amd64.img",
    #     download_url="https://cloud-images.ubuntu.com/focal/current/focal-server-cloudimg-amd64.img",
    #     os_variant="ubuntufocal",
    #     os_init=OSInit.cloud_init,
    # )

    Ubuntu22_04 = OSData(
        filename="jammy-server-cloudimg-amd64.img",
        download_url="https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img",
        os_variant="ubuntujammy",
        os_init=OSInit.cloud_init,
    )

    # Ubuntu24_04 = OSData(
    #     filename="noble-server-cloudimg-amd64.img",
    #     download_url="https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img",
    #     os_variant="ubuntunoble",
    #     os_init=OSInit.cloud_init,
    # )

    def download_image(self):
        # download the image into the workspace for the harness in the testbed libvirt images folder
        if self.exists():
            return
        logging.info(f"Downloading image: {self}")
        urlretrieve(self.value.download_url, self.get_base_img_location())

        # TODO
        self.check_hash()

    def get_base_img_location(self) -> str:
        return f"{harness_settings.workspace}/{self.value.filename}"

    def exists(self) -> bool:
        # check if the image already exists on disk
        image = Path(f"{harness_settings.workspace}/{self.value.filename}")
        if image.is_file():
            logging.info(f"{self.value.filename} exists: {image}")
            return True
        return False

    def check_hash(self) -> bool:
        # TODO - we should really evaluate the checksums of the existing image to the image online, to also check if
        #  potentially the image did not download properly - ubuntu provide the checksum list.
        #  However, we would need to store specific info on how/where to get the checksum for different image providers
        #  we might have to ditch the enum or insert more data into the enum to contain:
        #    1) download URL
        #    2) hash URL
        #    3) and at least for ubuntu, the name of the file to get the hash from step 2) as it contains all hashes
        logging.warn("Check hash of download not yet implemented")



