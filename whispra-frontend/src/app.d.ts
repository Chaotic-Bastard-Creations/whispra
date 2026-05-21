declare const __WHISPRA_SECURITY_MD__: string;
declare const __WHISPRA_BUILD_COMMIT__: string;
declare const __WHISPRA_BUILD_TAG__: string;

declare module "qrcode" {
  export interface QRCodeToDataURLOptions {
    width?: number;
    margin?: number;
    color?: {
      dark?: string;
      light?: string;
    };
  }

  const QRCode: {
    toDataURL(text: string, options?: QRCodeToDataURLOptions): Promise<string>;
  };

  export default QRCode;
}

declare namespace App {}
