/**************************************************************************
 *   SiUSBXp.c                                                            *
 *                                                                        *
 *   This library provides an API compatible with SiUSBXp.dll supplied    *
 *   with SiLabs USBXpress, except that this library has a libusb         *
 *   back-end and is therefore cross-platform (including Linux).          *
 *   This library has been proven to function correctly with .NET         *
 *   applications running under MONO.                                     *
 *   This implementation is incomplete, however the most commonly used    *
 *   functions are implemented.                                           *
 *                                                                        *
 *   Home Page:                                                           *
 *   http://www.etheus.net/SiUSBXp_Linux_Driver                           *
 *                                                                        *
 *   Compile with:                                                        *
 *   gcc -shared -lusb -o libSiUSBXp.so SiUSBXp.c -Wall -fPIC             *
 *                                                                        *
 *   Copyright (C) 2010 Craig Shelley <craig@microtron.org.uk>            *
 *   This program is free software; you can redistribute it and/or modify *
 *   it under the terms of the GNU General Public License as published by *
 *   the Free Software Foundation; either version 2, or (at your option)  *
 *   any later version.                                                   *
 *                                                                        *
 *   This program is distributed in the hope that it will be useful,      *
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of       *
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the        *
 *   GNU General Public License for more details.                         *
 *                                                                        *
 *   You should have received a copy of the GNU General Public License    *
 *   along with this program; if not, write to the Free Software          *
 *   Foundation, Inc., 675 Mass Ave, Cambridge, MA 02139, USA.            *
 *                                                                        *
 *   Credits:                                                             *
 *   Many thanks to Michael Heubeck for all of the testing, debugging     * 
 *   feedback, and for the original project idea.                         *
 *                                                                        *
 *   Version History:                                                     *
 *   0.01        2010-10-20    Initial release                            *
 *                             Craig Shelley <craig@microtron.org.uk>     *
 *               2020-06-28    Modified by Guoli Lyu
 *                                                                        *
 **************************************************************************/

/*Un-comment this line to see debug info*/
//#define DEBUG

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>

#if defined(_WIN32) || defined(WIN32) 
#include <lusb0_usb.h>
#else
#include <unistd.h>
#include <usb.h>
#endif


/*Vendor ID / Product ID*/
#define SI_USB_VID 0x10c4
#define SI_USB_PID 0x8149

/*Return codes*/
#define SI_SUCCESS 0x00
#define SI_DEVICE_NOT_FOUND 0xFF
#define SI_INVALID_HANDLE 0x01
#define SI_READ_ERROR 0x02
#define SI_RX_QUEUE_NOT_READY 0x03
#define SI_WRITE_ERROR 0x04
#define SI_RESET_ERROR 0x05
#define SI_INVALID_PARAMETER 0x06
#define SI_INVALID_REQUEST_LENGTH 0x07
#define SI_DEVICE_IO_FAILED 0x08
#define SI_INVALID_BAUDRATE 0x09
#define SI_FUNCTION_NOT_SUPPORTED 0x0a
#define SI_GLOBAL_DATA_ERROR 0x0b
#define SI_SYSTEM_ERROR_CODE 0x0c
#define SI_READ_TIMED_OUT 0x0d
#define SI_WRITE_TIMED_OUT 0x0e
#define SI_IO_PENDING 0x0f

/*GetProductString() function flags*/
#define SI_RETURN_SERIAL_NUMBER 0x00
#define SI_RETURN_DESCRIPTION 0x01
#define SI_RETURN_LINK_NAME 0x02
#define SI_RETURN_VID 0x03
#define SI_RETURN_PID 0x04

/*RX Queue status flags*/
#define SI_RX_NO_OVERRUN 0x00
#define SI_RX_EMPTY 0x00
#define SI_RX_OVERRUN 0x01
#define SI_RX_READY 0x02

/*Buffer size limits*/
#define SI_MAX_DEVICE_STRLEN 256
#define SI_MAX_READ_SIZE 4096 * 16
#define SI_MAX_WRITE_SIZE 4096

#if defined(_WIN32) || defined(WIN32) 
    #define ERR(format, ...) fprintf (stderr, format, __VA_ARGS__)
    #ifdef DEBUG
        #define DBG(format, ...) fprintf (stderr, format, __VA_ARGS__)
    #else
        #define DBG(format, ...)
    #endif
#else
    #define ERR(args...) printf(args)
    #ifdef DEBUG
        #define DBG(args...) printf(args)
    #else
        #define DBG(format, ...)
    #endif
#endif

#define MAGIC 12939485
#define BUF_SIZE 4096

int RXTimeout = 1000;
int TXTimeout = 1000;

int USBInitialised = 0;
struct usb_bus *busses;

struct SI_Private {
    int magic;
    usb_dev_handle *udev;
    int interface;
    int ep_out;
    int ep_in;
    int bufsize;
    char buffer[BUF_SIZE];
};

void init(void) {
    if (!USBInitialised) {
        DBG("Initialising USB\n");
        usb_init();

        USBInitialised = 1;
    }
}

int SI_GetNumDevices(int *NumDevices) {
    struct usb_bus *bus;
    struct usb_device *dev;
    int devcount;

    DBG("SI_GetNumDevices()\n");
    init();

    usb_find_busses();
    usb_find_devices();

    busses = usb_get_busses();

    devcount = 0;
    for (bus = busses; bus; bus = bus->next) {
        for (dev = bus->devices; dev; dev = dev->next) {
            devcount++;
        }
    }
    *NumDevices = devcount;

    DBG("  NumDevices=%i\n", *NumDevices);
    return SI_SUCCESS;
}

int SI_GetProductString(int DeviceNum, char *DeviceString, int Flags) {
    struct usb_bus *bus;
    struct usb_device *dev, *pdev;
    usb_dev_handle *udev;
    int devcount;
    int ret, descriptor;
    char tbuf[256];

    DBG("SI_GetProductString(DeviceNum=%i, DeviceString=%p, Flags=%i)\n", DeviceNum, DeviceString, Flags);
    init();

    if (DeviceString == NULL)
        return SI_INVALID_PARAMETER;

    strcpy(DeviceString, "");

    devcount = 0;
    pdev = NULL;
    for (bus = busses; bus; bus = bus->next) {
        for (dev = bus->devices; dev; dev = dev->next) {
            if (devcount == DeviceNum)
                pdev = dev;
            devcount++;
        }
    }

    if (pdev != NULL) {
        switch (Flags) {
            case SI_RETURN_SERIAL_NUMBER:
                descriptor = 3;
                break;
            case SI_RETURN_DESCRIPTION:
                descriptor = 2;
                break;
            case SI_RETURN_LINK_NAME:
                descriptor = 1;
                break;
            case SI_RETURN_VID: {
                char buffer[5];
                sprintf(buffer, "%x", pdev->descriptor.idVendor);
                strcpy(DeviceString, buffer);
                return SI_SUCCESS;
            }
            case SI_RETURN_PID: {
                char buffer[5];
                sprintf(buffer, "%x", pdev->descriptor.idProduct);
                strcpy(DeviceString, buffer);
            }
                return SI_SUCCESS;
            default:
                return SI_INVALID_PARAMETER;
        };
        DBG("  Vendor=0x%04X Product=0x%04X\n", pdev->descriptor.idVendor, pdev->descriptor.idProduct);
        if (pdev->descriptor.idVendor == SI_USB_VID &&
            pdev->descriptor.idProduct == SI_USB_PID) {
            DBG("  Device Found!\n");

            udev = usb_open(pdev);
            if (udev) {
                ret = usb_get_string_simple(udev, descriptor, tbuf, sizeof(tbuf));
                if (ret > 0) {
                    DBG("  Descriptor[%i]=\"%s\"\n", descriptor, tbuf);
                    strcpy(DeviceString, tbuf);
                } else {
                    ERR("  Unable to read Descriptor[%i]\n", descriptor);
                }
                usb_close(udev);
            } else {
                ERR("  Unable to open USB device\n");
            }
        }
    } else {
        return SI_DEVICE_NOT_FOUND;
    }

    DBG("  DeviceString=\"%s\"\n", DeviceString);

    return SI_SUCCESS;
}

static void SI_FillBuffer(struct SI_Private *Handle, int timeout) {
    int bytestoread, nread;
    bytestoread = BUF_SIZE - Handle->bufsize;
    DBG("  SI_FillBuffer BytesToRead=%i\n", bytestoread);
    nread = usb_bulk_read(Handle->udev, Handle->ep_in, &(Handle->buffer[Handle->bufsize]), bytestoread, timeout);
    DBG("  SI_FillBuffer Read=%i\n", nread);
    if (nread > 0) {
        Handle->bufsize += nread;
    }
    DBG("  SI_FillBuffer Handle->bufsize=%i\n", Handle->bufsize);
}

static int SI_GetBuffer(struct SI_Private *Handle, char *Buffer, int BytesToGet) {
    int retval;

    retval = 0;

    DBG("  SI_GetBuffer BytesToGet=%i Handle->bufsize=%i\n", BytesToGet, Handle->bufsize);
    if (Handle->bufsize >= BytesToGet) {
        retval = BytesToGet;
        Handle->bufsize -= retval;
        memcpy(Buffer, Handle->buffer, retval);
        memmove(Handle->buffer, Handle->buffer + retval, Handle->bufsize);
    } else if (Handle->bufsize > 0) {
        retval = Handle->bufsize;
        Handle->bufsize = 0;
        memcpy(Buffer, Handle->buffer, retval);
    }
    DBG("  SI_GetBuffer retval=%i Handle->bufsize=%i\n", retval, Handle->bufsize);

    return retval;
}

int SI_Open(int DeviceNum, struct SI_Private **pHandle) {
    struct usb_bus *bus;
    struct usb_device *dev, *pdev;
    struct SI_Private *Handle;
    int devcount;
    int i;
    DBG("SI_Open(DeviceNum=%i, pHandle=%p)\n", DeviceNum, pHandle);
    init();

    if (pHandle == NULL)
        return SI_INVALID_PARAMETER;

    /*Find the device*/
    devcount = 0;
    pdev = NULL;
    for (bus = busses; bus; bus = bus->next) {
        for (dev = bus->devices; dev; dev = dev->next) {
            if (devcount == DeviceNum)
                pdev = dev;
            devcount++;
        }
    }

    Handle = NULL;
    if (pdev != NULL) {
        Handle = (struct SI_Private *) malloc(sizeof(struct SI_Private));
    }

    /*Find the bulk in/out endpoints*/
    if (Handle != NULL) {
        Handle->ep_out = -1;
        Handle->ep_in = -1;
        for (i = 0; i < pdev->config[0].interface[0].altsetting[0].bNumEndpoints; i++) {
            if (pdev->config[0].interface[0].altsetting[0].endpoint[i].bmAttributes == USB_ENDPOINT_TYPE_BULK) {
                if ((pdev->config[0].interface[0].altsetting[0].endpoint[i].bEndpointAddress & USB_ENDPOINT_DIR_MASK) !=
                    0) {
                    Handle->ep_in = pdev->config[0].interface[0].altsetting[0].endpoint[i].bEndpointAddress;
                } else {
                    Handle->ep_out = pdev->config[0].interface[0].altsetting[0].endpoint[i].bEndpointAddress;
                }
            }
        }
        DBG("  EP_Out=0x%02x  EP_In=0x%02x\n", Handle->ep_out, Handle->ep_in);
        if (Handle->ep_out == -1 || Handle->ep_in == -1) {
            free(Handle);
            Handle = NULL;
            ERR("  **ERROR** Unable to identify BULK IN/OUT endpoints\n");
        }
    }

    if (Handle != NULL) {
        Handle->udev = usb_open(pdev);
        if (Handle->udev == NULL) {
            free(Handle);
            Handle = NULL;
            ERR("  **ERROR** Unable to open USB device\n");
        }
    }

    /*Claim the interface*/
    if (Handle != NULL) {
        Handle->interface = pdev->config[0].interface[0].altsetting[0].bInterfaceNumber;
        if (usb_claim_interface(Handle->udev, Handle->interface)) {
            usb_close(Handle->udev);
            free(Handle);
            Handle = NULL;
            ERR("  **ERROR** Unable to claim interface. Ensure device is not claimed by any kernel modules. Check permissions of /dev/bus/usb/...\n");
        }
    }

    if (Handle != NULL) {
        DBG("  USB Ctrl Message1 retval=%i\n",
            usb_control_msg(Handle->udev, 0x40, 0x00, 0xFFFF, 0, NULL, 0, TXTimeout));
        DBG("  USB Reset Endpoint IN retval=%i\n", usb_resetep(Handle->udev, Handle->ep_in));
        DBG("  USB Reset Endpoint OUT retval=%i\n", usb_resetep(Handle->udev, Handle->ep_out));
        DBG("  USB Clear Halt IN retval=%i\n", usb_clear_halt(Handle->udev, Handle->ep_in));
        DBG("  USB Clear Halt OUT retval=%i\n", usb_clear_halt(Handle->udev, Handle->ep_out));
        DBG("  USB Ctrl Message2 retval=%i\n",
            usb_control_msg(Handle->udev, 0x40, 0x02, 0x0002, 0, NULL, 0, TXTimeout));

        Handle->bufsize = 0;

        SI_FillBuffer(Handle, 100);

        Handle->magic = MAGIC;
        *pHandle = Handle;

        DBG("  Success!\n");

        return SI_SUCCESS;
    } else {
        return SI_SYSTEM_ERROR_CODE;
    }
}

int SI_Close(struct SI_Private *Handle) {
    DBG("SI_Close(Handle=%p)\n", Handle);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");
    DBG("  USB Ctrl Message retval=%i\n", usb_control_msg(Handle->udev, 0x40, 0x02, 0x0004, 0, NULL, 0, TXTimeout));

    usb_release_interface(Handle->udev, Handle->interface);
    usb_close(Handle->udev);

    Handle->magic = 0;
    free(Handle);

    return SI_SUCCESS;
}

int SI_Read(struct SI_Private *Handle, char *Buffer, int BytesToRead, int *BytesReturned, void *o) {
    int i;
    DBG("SI_Read(Handle=%p, Buffer=%p, BytesToRead=%i, BytesReturned=%p)\n", Handle, Buffer, BytesToRead,
        BytesReturned);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    if (Buffer == NULL)
        return SI_INVALID_PARAMETER;
    if (Buffer == NULL || BytesReturned == NULL)
        return SI_INVALID_PARAMETER;

    if (Handle->bufsize < BytesToRead)
        SI_FillBuffer(Handle, RXTimeout);
    *BytesReturned = SI_GetBuffer(Handle, Buffer, BytesToRead);
    DBG("  ReadBytes \"");
    for (i = 0; i < *BytesReturned; i++) {
        if (i > 0) {
            DBG(",");
        }
        DBG("%02X", (unsigned char) Buffer[i]);
    }
    DBG("\"\n");
    DBG("  Read %i bytes\n", *BytesReturned);

    return *BytesReturned > 0 ? SI_SUCCESS : SI_READ_TIMED_OUT;
}

int SI_Write(struct SI_Private *Handle, char *Buffer, int BytesToWrite, int *BytesWritten, void *o) {
    int i;
    DBG("SI_Write(Handle=%p, Buffer=%p, BytesToWrite=%i, BytesWritten=%p)\n", Handle, Buffer, BytesToWrite,
        BytesWritten);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    if (Buffer == NULL || BytesWritten == NULL)
        return SI_INVALID_PARAMETER;

    DBG("  Writing \"");
    for (i = 0; i < BytesToWrite; i++) {
        if (i > 0) {
            DBG(",");
        }
        DBG("%02X", (unsigned char) Buffer[i]);
    }
    DBG("\"\n");
    SI_FillBuffer(Handle, 100);
    DBG("  Writing to device...\n");
    *BytesWritten = usb_bulk_write(Handle->udev, Handle->ep_out, Buffer, BytesToWrite, TXTimeout);
    SI_FillBuffer(Handle, 100);
    DBG("  Wrote %i bytes\n", *BytesWritten);

    return SI_SUCCESS;
}

int SI_ResetDevice(struct SI_Private *Handle) {
    DBG("SI_ResetDevice(Handle=%p)\n", Handle);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    return SI_SUCCESS;
}

int SI_DeviceIOControl(struct SI_Private *Handle, int IoControlCode, char *InBuffer, int BytesToRead, char *OutBuffer,
                       int BytesToWrite) {
    DBG("SI_DeviceIOControl(Handle=%p, IoControlCode=%i, InBuffer=%p, BytesToRead=%i, OutBuffer=%p, BytesToWrite=%i)\n",
        Handle, IoControlCode, InBuffer, BytesToRead, OutBuffer, BytesToWrite);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    return SI_SUCCESS;
}


int SI_FlushBuffers(struct SI_Private *Handle, char FlushTransmit, char FlushReceive) {
    DBG("SI_FlushTransmit(Handle=%p, FlushTransmit=%i, FlushReceive=%i)\n", Handle, FlushTransmit, FlushReceive);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    Handle->bufsize = 0;

    return SI_SUCCESS;
}

int SI_SetTimeouts(int ReadTimeout, int WriteTimeout) {
    DBG("SI_SetTimeouts(ReadTimeout=%i, WriteTimeout=%i)\n", ReadTimeout, WriteTimeout);
    init();

    RXTimeout = ReadTimeout;
    TXTimeout = WriteTimeout;

    return SI_SUCCESS;
}

int SI_GetTimeouts(int *ReadTimeout, int *WriteTimeout) {
    DBG("SI_GetTimeouts(ReadTimeout=%p, WriteTimeout=%p)\n", ReadTimeout, WriteTimeout);
    init();

    if (ReadTimeout == NULL || WriteTimeout == NULL)
        return SI_INVALID_PARAMETER;

    *ReadTimeout = RXTimeout;
    *WriteTimeout = TXTimeout;

    return SI_SUCCESS;
}

int SI_CheckRXQueue(struct SI_Private *Handle, int *NumBytesInQueue, int *QueueStatus) {
    DBG("SI_CheckRXQueue(Handle=%p, NumBytesInQueue=%p, QueueStatus=%p)\n", Handle, NumBytesInQueue, QueueStatus);
    init();

    if (Handle == NULL)
        return SI_INVALID_HANDLE;
    if (Handle->magic != MAGIC)
        return SI_INVALID_HANDLE;
    DBG("  Valid Handle\n");

    if (NumBytesInQueue == NULL || QueueStatus == NULL)
        return SI_INVALID_PARAMETER;

    *NumBytesInQueue = Handle->bufsize;
    *QueueStatus = SI_RX_NO_OVERRUN | (Handle->bufsize ? SI_RX_READY : SI_RX_EMPTY);

    DBG("  NumBytesInQueue=%i QueueStatus=%i\n", *NumBytesInQueue, *QueueStatus);

    return SI_SUCCESS;
}
